use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, Method, StatusCode};
use serde_json::Value;
use tokio::sync::RwLock;

use super::models::*;
use super::protocol::*;
use super::scramble;

impl From<String> for JmError {
    fn from(value: String) -> Self {
        JmError::Protocol(value)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JmError {
    #[error("网络请求失败: {0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("接口返回 code={code}: {message}")]
    Api { code: i64, message: String },
    #[error("未登录或登录已失效")]
    Auth,
    #[error("协议错误: {0}")]
    Protocol(String),
    #[error("解析错误: {0}")]
    Parse(String),
}

fn build_client(proxy: Option<&str>) -> Result<Client, reqwest::Error> {
    let mut builder = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(40));
    if let Some(url) = proxy {
        if !url.trim().is_empty() {
            builder = builder.proxy(reqwest::Proxy::all(url.trim())?);
        }
    }
    builder.build()
}

fn value_i64(value: &Value) -> i64 {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|v| v.trim().parse().ok()))
        .unwrap_or(0)
}

fn value_f64(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|v| v.trim().parse().ok()))
        .unwrap_or(0.0)
}

fn value_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(item) = value.get(*key) {
            let text = value_string(item);
            if !text.trim().is_empty() && text != "null" {
                return Some(text);
            }
        }
    }
    None
}

fn avatar_url(img_host: &str, photo: &str) -> Option<Thumb> {
    let photo = photo.trim();
    if photo.is_empty() || photo == "nopic-Male.gif" || photo == "null" {
        return None;
    }
    let path = if photo.starts_with("http://") || photo.starts_with("https://") {
        photo.to_string()
    } else {
        format!("{}/media/users/{}", normalize_host(img_host), photo)
    };
    Some(Thumb {
        path,
        file_server: String::new(),
    })
}

fn profile_from_value(value: &Value, img_host: &str) -> ProfileUser {
    let uid = first_string(value, &["uid", "_id", "id"]).unwrap_or_default();
    let photo = first_string(value, &["photo", "avatar"]).unwrap_or_default();
    ProfileUser {
        id: uid,
        name: first_string(value, &["username", "name", "nickname"]).unwrap_or_default(),
        email: first_string(value, &["email"]),
        level: value.get("level").map(value_i64).unwrap_or(0),
        level_name: first_string(value, &["level_name", "levelName"]).unwrap_or_default(),
        exp: value.get("exp").map(value_i64).unwrap_or(0),
        exp_percent: value.get("exp_percent").map(value_f64).unwrap_or(0.0),
        coin: value.get("coin").map(value_i64).unwrap_or(0),
        avatar: avatar_url(img_host, &photo),
        favourite_count: value.get("album_favorites").map(value_i64).unwrap_or(0),
        max_favourite_count: value
            .get("album_favorites_max")
            .map(value_i64)
            .unwrap_or(0),
        is_punched: value
            .get("is_punched")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn action_from_value(value: &Value) -> ActionResult {
    ActionResult {
        status: first_string(value, &["status"]).unwrap_or_default(),
        message: first_string(value, &["msg", "message"]).unwrap_or_default(),
        action_type: first_string(value, &["type"]).unwrap_or_default(),
    }
}

#[derive(Clone)]
pub struct JmClient {
    client: Arc<RwLock<Client>>,
    proxy: Arc<RwLock<Option<String>>>,
    api_domain: Arc<RwLock<String>>,
    api_domains: Arc<RwLock<Vec<String>>>,
    api_version: Arc<RwLock<String>>,
    img_host: Arc<RwLock<String>>,
    avs: Arc<RwLock<Option<String>>>,
    login_domain: Arc<RwLock<Option<String>>>,
    user_id: Arc<RwLock<Option<String>>>,
    scramble_cache: Arc<RwLock<HashMap<String, i64>>>,
}

impl JmClient {
    pub fn new() -> Self {
        let client = build_client(None).expect("Failed to build JM HTTP client");
        Self {
            client: Arc::new(RwLock::new(client)),
            proxy: Arc::new(RwLock::new(None)),
            api_domain: Arc::new(RwLock::new(API_DOMAINS[0].to_string())),
            api_domains: Arc::new(RwLock::new(
                API_DOMAINS.iter().map(|s| s.to_string()).collect(),
            )),
            api_version: Arc::new(RwLock::new(DEFAULT_API_VERSION.to_string())),
            img_host: Arc::new(RwLock::new("cdn-msp2.jmapiproxy2.cc".to_string())),
            avs: Arc::new(RwLock::new(None)),
            login_domain: Arc::new(RwLock::new(None)),
            user_id: Arc::new(RwLock::new(None)),
            scramble_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set_proxy(&self, proxy: Option<String>) -> Result<(), JmError> {
        let cleaned = proxy.filter(|p| !p.trim().is_empty());
        let client = build_client(cleaned.as_deref())?;
        *self.client.write().await = client;
        *self.proxy.write().await = cleaned;
        Ok(())
    }

    pub async fn get_proxy(&self) -> Option<String> {
        self.proxy.read().await.clone()
    }

    pub async fn set_session(
        &self,
        avs: Option<String>,
        domain: Option<String>,
        user_id: Option<String>,
    ) {
        *self.avs.write().await = avs.filter(|s| !s.trim().is_empty());
        *self.login_domain.write().await = domain.filter(|s| !s.trim().is_empty());
        *self.user_id.write().await = user_id.filter(|s| !s.trim().is_empty());
        if let Some(domain) = self.login_domain.read().await.clone() {
            *self.api_domain.write().await = domain;
        }
    }

    pub async fn clear_session(&self) {
        *self.avs.write().await = None;
        *self.login_domain.write().await = None;
        *self.user_id.write().await = None;
    }

    pub async fn avs(&self) -> Option<String> {
        self.avs.read().await.clone()
    }

    pub async fn login_domain(&self) -> Option<String> {
        self.login_domain.read().await.clone()
    }

    pub async fn user_id(&self) -> Option<String> {
        self.user_id.read().await.clone()
    }

    pub async fn is_logged_in(&self) -> bool {
        self.avs.read().await.is_some()
    }

    pub async fn api_domain(&self) -> String {
        self.api_domain.read().await.clone()
    }

    pub async fn api_domains(&self) -> Vec<String> {
        self.api_domains.read().await.clone()
    }

    pub async fn api_version(&self) -> String {
        self.api_version.read().await.clone()
    }

    pub async fn img_host(&self) -> String {
        self.img_host.read().await.clone()
    }

    pub async fn set_img_host(&self, host: String) {
        if !host.trim().is_empty() {
            *self.img_host.write().await = normalize_host(&host);
        }
    }

    pub async fn set_api_domain(&self, domain: &str) -> Result<(), JmError> {
        let domain = domain
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        if domain.is_empty() {
            return Err(JmError::Protocol("API 域名不能为空".to_string()));
        }
        let login = self.login_domain.read().await.clone();
        *self.api_domain.write().await = domain.clone();
        if let Some(login) = login {
            if login != domain {
                self.clear_session().await;
            }
        }
        Ok(())
    }

    pub async fn bootstrap(&self) -> Result<(), JmError> {
        let value = self
            .request_value(
                Method::GET,
                "/setting",
                None,
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        if let Some(version) = first_string(&value, &["jm3_version", "version", "api_version"]) {
            *self.api_version.write().await = version;
        }
        if let Some(host) = first_string(&value, &["img_host", "imgHost"]) {
            *self.img_host.write().await = normalize_host(&host);
        }
        Ok(())
    }

    pub async fn refresh_api_domains(&self) -> Result<Vec<String>, JmError> {
        let client = self.client.read().await.clone();
        let mut last_error = None;
        for server in DOMAIN_SERVERS {
            let response = match client
                .get(*server)
                .header("user-agent", USER_AGENT)
                .send()
                .await
            {
                Ok(response) => response,
                Err(error) => {
                    last_error = Some(JmError::Http(error));
                    continue;
                }
            };
            let text = match response.text().await {
                Ok(text) => text,
                Err(error) => {
                    last_error = Some(JmError::Http(error));
                    continue;
                }
            };
            let decrypted = match decrypt_domain_server(&text) {
                Ok(text) => text,
                Err(error) => {
                    last_error = Some(JmError::Protocol(error));
                    continue;
                }
            };
            let value: Value = match serde_json::from_str(&decrypted) {
                Ok(value) => value,
                Err(error) => {
                    last_error = Some(JmError::Parse(error.to_string()));
                    continue;
                }
            };
            let Some(servers) = value.get("Server").and_then(Value::as_array) else {
                continue;
            };
            let mut domains = Vec::new();
            for server in servers {
                if let Some(host) = server.as_str() {
                    let host = host
                        .trim()
                        .trim_start_matches("https://")
                        .trim_start_matches("http://")
                        .trim_end_matches('/')
                        .to_string();
                    if !host.is_empty() && !domains.contains(&host) {
                        domains.push(host);
                    }
                }
            }
            if !domains.is_empty() {
                *self.api_domains.write().await = domains.clone();
                return Ok(domains);
            }
        }
        Err(last_error.unwrap_or_else(|| JmError::Protocol("域名服务器未返回可用域名".to_string())))
    }

    async fn candidate_domains(&self, _path: &str) -> Vec<String> {
        if let Some(login) = self.login_domain.read().await.clone() {
            return vec![login];
        }
        let current = self.api_domain.read().await.clone();
        let mut domains = vec![current.clone()];
        for domain in self.api_domains.read().await.iter() {
            if domain != &current {
                domains.push(domain.clone());
            }
        }
        domains
    }

    async fn raw_request_on(
        &self,
        domain: &str,
        method: &Method,
        path: &str,
        query: Option<&[(String, String)]>,
        form: Option<&[(String, String)]>,
        secret: &str,
    ) -> Result<(String, String), JmError> {
        let ts = chrono::Utc::now().timestamp().to_string();
        let version = self.api_version.read().await.clone();
        let (token, tokenparam) = token_and_param(&ts, &version, secret);
        let url = format!("https://{domain}{path}");
        let client = self.client.read().await.clone();
        let mut request = client
            .request(method.clone(), &url)
            .header("token", token)
            .header("tokenparam", tokenparam)
            .header("user-agent", USER_AGENT);
        if let Some(query) = query {
            request = request.query(query);
        }
        if let Some(form) = form {
            request = request.form(form);
        }
        if let Some(avs) = self.avs.read().await.clone() {
            request = request.header("Cookie", format!("AVS={avs}"));
        }
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(JmError::Auth);
        }
        if !status.is_success() {
            return Err(JmError::HttpStatus {
                status: status.as_u16(),
                body: text,
            });
        }
        Ok((ts, text))
    }

    async fn request_value_on(
        &self,
        domain: &str,
        method: &Method,
        path: &str,
        query: Option<&[(String, String)]>,
        form: Option<&[(String, String)]>,
        secret: &str,
    ) -> Result<Value, JmError> {
        let (ts, text) = self
            .raw_request_on(domain, method, path, query, form, secret)
            .await?;
        let envelope: JmEnvelope = serde_json::from_str(&text).map_err(|error| {
            JmError::Parse(format!(
                "响应不是 JM JSON（{}）: {}",
                error,
                text.chars().take(300).collect::<String>()
            ))
        })?;
        if envelope.code != 200 {
            if envelope.code == 401 {
                return Err(JmError::Auth);
            }
            return Err(JmError::Api {
                code: envelope.code,
                message: envelope.error_msg,
            });
        }
        if envelope.data.is_null() {
            return Ok(Value::Null);
        }
        if let Some(encoded) = envelope.data.as_str() {
            if encoded.trim().is_empty() {
                return Ok(Value::Null);
            }
            let decrypted = decrypt_api_data(encoded, &ts)?;
            return serde_json::from_str(&decrypted)
                .map_err(|error| JmError::Parse(format!("解密结果不是 JSON: {error}")));
        }
        Ok(envelope.data)
    }

    async fn request_value(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(String, String)]>,
        form: Option<&[(String, String)]>,
        secret: &str,
        allow_fallback: bool,
    ) -> Result<Value, JmError> {
        let domains = self.candidate_domains(path).await;
        let mut last_error = None;
        for domain in domains {
            match self
                .request_value_on(&domain, &method, path, query, form, secret)
                .await
            {
                Ok(value) => {
                    *self.api_domain.write().await = domain;
                    return Ok(value);
                }
                Err(JmError::Auth) => return Err(JmError::Auth),
                Err(error) => {
                    last_error = Some(error);
                    if !allow_fallback {
                        break;
                    }
                }
            }
        }
        Err(last_error.unwrap_or_else(|| JmError::Protocol("没有可用 API 域名".to_string())))
    }

    async fn request_html(
        &self,
        path: &str,
        query: &[(String, String)],
        secret: &str,
    ) -> Result<String, JmError> {
        let domains = self.candidate_domains(path).await;
        let mut last_error = None;
        for domain in domains {
            match self
                .raw_request_on(&domain, &Method::GET, path, Some(query), None, secret)
                .await
            {
                Ok((_, text)) => {
                    *self.api_domain.write().await = domain;
                    return Ok(text);
                }
                Err(JmError::Auth) => return Err(JmError::Auth),
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| JmError::Protocol("没有可用 API 域名".to_string())))
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<ProfileUser, JmError> {
        let form = vec![
            ("username".to_string(), username.to_string()),
            ("password".to_string(), password.to_string()),
        ];
        let value = self
            .request_value(
                Method::POST,
                "/login",
                None,
                Some(&form),
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let avs = first_string(&value, &["s", "AVS", "avs"]).unwrap_or_default();
        if avs.trim().is_empty() {
            return Err(JmError::Api {
                code: 0,
                message: "登录响应缺少 AVS".to_string(),
            });
        }
        let uid = first_string(&value, &["uid", "_id", "id"]).unwrap_or_default();
        *self.avs.write().await = Some(avs);
        *self.user_id.write().await = Some(uid);
        let domain = self.api_domain.read().await.clone();
        *self.login_domain.write().await = Some(domain);
        let img_host = self.img_host.read().await.clone();
        Ok(profile_from_value(&value, &img_host))
    }

    pub async fn get_profile(&self) -> Result<ProfileUser, JmError> {
        let value = self
            .request_value(
                Method::POST,
                "/login",
                None,
                None,
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        let img_host = self.img_host.read().await.clone();
        Ok(profile_from_value(&value, &img_host))
    }

    pub async fn logout(&self) {
        self.clear_session().await;
    }

    pub async fn search(
        &self,
        keyword: &str,
        page: i64,
        order: &str,
        main_tag: i64,
        time: &str,
        year: Option<i64>,
        month: Option<i64>,
    ) -> Result<SearchResult, JmError> {
        let mut query = vec![
            ("main_tag".to_string(), main_tag.to_string()),
            ("search_query".to_string(), keyword.to_string()),
            ("page".to_string(), page.max(1).to_string()),
            ("o".to_string(), order.to_string()),
            ("t".to_string(), time.to_string()),
        ];
        if let Some(year) = year {
            query.push(("y".to_string(), year.to_string()));
        }
        if let Some(month) = month {
            query.push(("m".to_string(), month.to_string()));
        }
        let value = self
            .request_value(
                Method::GET,
                "/search",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let response: JmSearchResp = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("搜索响应解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        if let Some(aid) = response.redirect_aid {
            let album = self.album_raw(&aid.to_string()).await?;
            let comic = comic_from_album(album, &img_host);
            return Ok(SearchResult {
                search_query: response.search_query,
                search_type: response.search_type,
                total: 1,
                content: Vec::new(),
                redirect_album: Some(Box::new(comic)),
            });
        }
        Ok(SearchResult {
            search_query: response.search_query,
            search_type: response.search_type,
            total: response.total,
            content: response
                .content
                .into_iter()
                .map(|item| comic_from_summary(item, &img_host))
                .collect(),
            redirect_album: None,
        })
    }

    pub async fn categories(&self) -> Result<CategoriesResult, JmError> {
        let value = self
            .request_value(
                Method::GET,
                "/categories",
                None,
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let response: JmCategoriesResp = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("分类响应解析失败: {error}")))?;
        let categories = response
            .categories
            .into_iter()
            .filter(|item| !item.slug.trim().is_empty())
            .map(|item| Category {
                id: item.id,
                title: item.name,
                slug: item.slug,
                total_albums: item.total_albums.trim().parse().unwrap_or(0),
                sub_categories: item
                    .sub_categories
                    .into_iter()
                    .map(|sub| SubCategory {
                        id: sub.id,
                        title: sub.name,
                        slug: sub.slug,
                    })
                    .collect(),
            })
            .collect();
        Ok(CategoriesResult {
            categories,
            blocks: response.blocks,
        })
    }

    pub async fn category_comics(
        &self,
        page: i64,
        category: &str,
        order: &str,
        time: &str,
    ) -> Result<SearchResult, JmError> {
        let order_param = if time.is_empty() || time == "a" {
            order.to_string()
        } else {
            format!("{order}_{time}")
        };
        let query = vec![
            ("page".to_string(), page.max(1).to_string()),
            ("order".to_string(), String::new()),
            ("c".to_string(), category.to_string()),
            ("o".to_string(), order_param),
        ];
        let value = self
            .request_value(
                Method::GET,
                "/categories/filter",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let response: JmSearchResp = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("分类列表解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        Ok(SearchResult {
            search_query: response.search_query,
            search_type: "category".to_string(),
            total: response.total,
            content: response
                .content
                .into_iter()
                .map(|item| comic_from_summary(item, &img_host))
                .collect(),
            redirect_album: None,
        })
    }

    pub async fn weekly_info(&self) -> Result<WeeklyInfo, JmError> {
        let value = self
            .request_value(Method::GET, "/week", None, None, APP_TOKEN_SECRET, true)
            .await?;
        let response: JmWeekInfo = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("周刊索引解析失败: {error}")))?;
        Ok(WeeklyInfo {
            categories: response
                .categories
                .into_iter()
                .map(|item| WeekCategory {
                    id: item.id,
                    title: item.title,
                    time: item.time,
                })
                .collect(),
            types: response
                .types
                .into_iter()
                .map(|item| {
                    // 接口对韩漫类型返回空标题，这里补齐展示名。
                    let title = if item.title.trim().is_empty() {
                        match item.id.as_str() {
                            "manga" => "日漫".to_string(),
                            "another" => "其他".to_string(),
                            "hanman" => "韩漫".to_string(),
                            other => other.to_string(),
                        }
                    } else {
                        item.title
                    };
                    WeekType { id: item.id, title }
                })
                .collect(),
        })
    }

    pub async fn weekly_list(
        &self,
        page: i64,
        category_id: &str,
        type_id: &str,
    ) -> Result<WeekList, JmError> {
        let query = vec![
            ("page".to_string(), page.max(1).to_string()),
            ("id".to_string(), category_id.to_string()),
            ("type".to_string(), type_id.to_string()),
        ];
        let value = self
            .request_value(
                Method::GET,
                "/week/filter",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let response: JmWeekList = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("周刊列表解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        Ok(WeekList {
            total: response.total,
            list: response
                .list
                .into_iter()
                .map(|item| comic_from_summary(item, &img_host))
                .collect(),
        })
    }

    pub async fn album_raw(&self, album_id: &str) -> Result<JmAlbum, JmError> {
        let query = vec![("id".to_string(), album_id.to_string())];
        let value = self
            .request_value(
                Method::GET,
                "/album",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("漫画详情解析失败: {error}")))
    }

    pub async fn comic_bundle(&self, album_id: &str) -> Result<ComicBundle, JmError> {
        let album = self.album_raw(album_id).await?;
        let img_host = self.img_host.read().await.clone();
        let episodes = episodes_from_album(&album);
        Ok(ComicBundle {
            comic: comic_from_album(album, &img_host),
            episodes,
        })
    }

    pub async fn comic(&self, album_id: &str) -> Result<Comic, JmError> {
        Ok(self.comic_bundle(album_id).await?.comic)
    }

    pub async fn episodes(&self, album_id: &str) -> Result<Vec<Episode>, JmError> {
        Ok(self.comic_bundle(album_id).await?.episodes)
    }

    pub async fn chapter(&self, chapter_id: &str) -> Result<JmChapter, JmError> {
        let query = vec![("id".to_string(), chapter_id.to_string())];
        let value = self
            .request_value(
                Method::GET,
                "/chapter",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("章节详情解析失败: {error}")))
    }

    pub async fn scramble_id(&self, chapter_id: &str) -> i64 {
        if let Some(value) = self.scramble_cache.read().await.get(chapter_id).copied() {
            return value;
        }
        let ts = chrono::Utc::now().timestamp().to_string();
        let query = vec![
            ("id".to_string(), chapter_id.to_string()),
            ("app_img_shunt".to_string(), "1".to_string()),
            ("mode".to_string(), "vertical".to_string()),
            ("page".to_string(), "0".to_string()),
            ("express".to_string(), "off".to_string()),
            ("v".to_string(), ts),
        ];
        let parsed = match self
            .request_html("/chapter_view_template", &query, CHAPTER_TOKEN_SECRET)
            .await
        {
            Ok(text) if !text.contains("TPERROR") => text
                .split("var scramble_id = ")
                .nth(1)
                .and_then(|rest| rest.split(';').next())
                .and_then(|number| number.trim().parse::<i64>().ok()),
            _ => None,
        };
        if let Some(value) = parsed {
            self.scramble_cache
                .write()
                .await
                .insert(chapter_id.to_string(), value);
            value
        } else {
            220_980
        }
    }

    pub async fn forum(
        &self,
        album_id: &str,
        page: i64,
    ) -> Result<(Vec<CommentItem>, i64), JmError> {
        let query = vec![
            ("page".to_string(), page.max(1).to_string()),
            ("aid".to_string(), album_id.to_string()),
            ("mode".to_string(), "manhua".to_string()),
        ];
        self.forum_with_query(query).await
    }

    async fn forum_with_query(
        &self,
        query: Vec<(String, String)>,
    ) -> Result<(Vec<CommentItem>, i64), JmError> {
        let value = self
            .request_value(
                Method::GET,
                "/forum",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let response: JmForumResp = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("评论解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        Ok(comments_from_forum(response, &img_host))
    }

    pub async fn post_comment(
        &self,
        album_id: &str,
        content: &str,
        comment_id: Option<&str>,
        spoiler: bool,
    ) -> Result<ActionResult, JmError> {
        let mut form = vec![
            ("comment".to_string(), content.to_string()),
            ("aid".to_string(), album_id.to_string()),
            (
                "status".to_string(),
                if spoiler { "false" } else { "true" }.to_string(),
            ),
        ];
        if let Some(comment_id) = comment_id {
            if !comment_id.trim().is_empty() {
                form.push(("comment_id".to_string(), comment_id.to_string()));
            }
        }
        let value = self
            .request_value(
                Method::POST,
                "/comment",
                None,
                Some(&form),
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        Ok(action_from_value(&value))
    }

    pub async fn toggle_like(&self, album_id: &str) -> Result<ActionResult, JmError> {
        let form = vec![("id".to_string(), album_id.to_string())];
        let value = self
            .request_value(
                Method::POST,
                "/like",
                None,
                Some(&form),
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        Ok(action_from_value(&value))
    }

    pub async fn favorite(
        &self,
        page: i64,
        order: &str,
        folder_id: &str,
    ) -> Result<FavoriteResult, JmError> {
        let query = vec![
            ("page".to_string(), page.max(1).to_string()),
            ("o".to_string(), order.to_string()),
            ("folder_id".to_string(), folder_id.to_string()),
        ];
        let value = self
            .request_value(
                Method::GET,
                "/favorite",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        let response: JmFavoriteResp = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("收藏夹解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        Ok(FavoriteResult {
            list: response
                .list
                .into_iter()
                .map(|item| comic_from_summary(item, &img_host))
                .collect(),
            folders: response
                .folder_list
                .into_iter()
                .map(|folder| FavoriteFolder {
                    id: folder.id,
                    name: folder.name,
                    owner_user_id: folder.owner_user_id,
                })
                .collect(),
            total: response.total.trim().parse().unwrap_or(0),
            count: response.count,
        })
    }

    pub async fn toggle_favorite(&self, album_id: &str) -> Result<ActionResult, JmError> {
        let form = vec![("aid".to_string(), album_id.to_string())];
        let value = self
            .request_value(
                Method::POST,
                "/favorite",
                None,
                Some(&form),
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        Ok(action_from_value(&value))
    }

    pub async fn move_favorite(
        &self,
        album_id: &str,
        folder_id: &str,
    ) -> Result<ActionResult, JmError> {
        let form = vec![
            ("type".to_string(), "move".to_string()),
            ("folder_id".to_string(), folder_id.to_string()),
            ("aid".to_string(), album_id.to_string()),
        ];
        let value = self
            .request_value(
                Method::POST,
                "/favorite_folder",
                None,
                Some(&form),
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        Ok(action_from_value(&value))
    }

    pub async fn promote(&self) -> Result<Vec<PromoteSection>, JmError> {
        let ts = chrono::Utc::now().timestamp().to_string();
        let query = vec![("_".to_string(), ts)];
        let value = self
            .request_value(
                Method::GET,
                "/promote",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let sections: Vec<JmPromoteSection> = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("推荐位解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        Ok(sections
            .into_iter()
            .map(|section| PromoteSection {
                id: section.id,
                title: section.title,
                slug: section.slug,
                section_type: section.section_type,
                filter_value: section.filter_value,
                content: section
                    .content
                    .into_iter()
                    .map(|item| comic_from_summary(item, &img_host))
                    .collect(),
            })
            .collect())
    }

    pub async fn promote_list(&self, section_id: &str, page: i64) -> Result<PromoteList, JmError> {
        let query = vec![
            ("id".to_string(), section_id.to_string()),
            ("page".to_string(), page.max(1).to_string()),
        ];
        let value = self
            .request_value(
                Method::GET,
                "/promote_list",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let response: JmWeekList = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("推荐分区解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        Ok(PromoteList {
            total: response.total,
            list: response
                .list
                .into_iter()
                .map(|item| comic_from_summary(item, &img_host))
                .collect(),
        })
    }

    pub async fn watch_list(&self, page: i64) -> Result<WatchListResult, JmError> {
        let query = vec![("page".to_string(), page.max(1).to_string())];
        let value = self
            .request_value(
                Method::GET,
                "/watch_list",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        let response: JmWeekList = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("观看历史解析失败: {error}")))?;
        let img_host = self.img_host.read().await.clone();
        Ok(WatchListResult {
            total: response.total,
            list: response
                .list
                .into_iter()
                .map(|item| comic_from_summary(item, &img_host))
                .collect(),
        })
    }

    pub async fn daily(&self, user_id: &str) -> Result<DailyInfo, JmError> {
        let query = vec![("user_id".to_string(), user_id.to_string())];
        let value = self
            .request_value(
                Method::GET,
                "/daily",
                Some(&query),
                None,
                APP_TOKEN_SECRET,
                true,
            )
            .await?;
        let response: JmDailyInfo = serde_json::from_value(value)
            .map_err(|error| JmError::Parse(format!("签到信息解析失败: {error}")))?;
        let mut records = Vec::new();
        for group in response.record {
            for item in group {
                records.push(DailyRecord {
                    date: item.date,
                    signed: item.signed,
                    bonus: item.bonus,
                });
            }
        }
        Ok(DailyInfo {
            daily_id: response.daily_id,
            event_name: response.event_name,
            current_progress: response.current_progress,
            three_days_coin: response.three_days_coin,
            three_days_exp: response.three_days_exp,
            seven_days_coin: response.seven_days_coin,
            seven_days_exp: response.seven_days_exp,
            records,
        })
    }

    pub async fn daily_checkin(
        &self,
        user_id: &str,
        daily_id: &str,
    ) -> Result<ActionResult, JmError> {
        let form = vec![
            ("user_id".to_string(), user_id.to_string()),
            ("daily_id".to_string(), daily_id.to_string()),
        ];
        let value = self
            .request_value(
                Method::POST,
                "/daily_chk",
                None,
                Some(&form),
                APP_TOKEN_SECRET,
                false,
            )
            .await?;
        Ok(action_from_value(&value))
    }

    pub async fn image_bytes_direct(&self, url: &str) -> Result<Vec<u8>, JmError> {
        let client = self.client.read().await.clone();
        let response = client
            .get(url)
            .header("user-agent", USER_AGENT)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            return Err(JmError::HttpStatus {
                status: status.as_u16(),
                body: url.to_string(),
            });
        }
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    pub async fn image_bytes(&self, chapter_id: &str, filename: &str) -> Result<Vec<u8>, JmError> {
        let current = self.img_host.read().await.clone();
        let mut hosts = vec![current];
        for host in IMAGE_HOSTS {
            let host = host.to_string();
            if !hosts.contains(&host) {
                hosts.push(host);
            }
        }
        let referer = format!("https://{}/", self.api_domain.read().await);
        let mut last_error = None;
        for host in hosts {
            let url = photo_url(&host, chapter_id, filename);
            let client = self.client.read().await.clone();
            let response = client
                .get(&url)
                .header("user-agent", USER_AGENT)
                .header("referer", &referer)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => {
                    match response.bytes().await {
                        Ok(bytes) if !bytes.is_empty() => return Ok(bytes.to_vec()),
                        Ok(_) => {
                            let ts = chrono::Utc::now().timestamp();
                            let retry_url = format!("{url}?v={ts}");
                            if let Ok(response) = client
                                .get(&retry_url)
                                .header("user-agent", USER_AGENT)
                                .header("referer", &referer)
                                .send()
                                .await
                            {
                                if let Ok(bytes) = response.bytes().await {
                                    if !bytes.is_empty() {
                                        return Ok(bytes.to_vec());
                                    }
                                }
                            }
                            last_error = Some(JmError::Protocol(format!("图片为空: {url}")));
                        }
                        Err(error) => last_error = Some(JmError::Http(error)),
                    }
                }
                Ok(response) => {
                    last_error = Some(JmError::HttpStatus {
                        status: response.status().as_u16(),
                        body: url,
                    });
                }
                Err(error) => last_error = Some(JmError::Http(error)),
            }
        }
        Err(last_error.unwrap_or_else(|| JmError::Protocol("没有可用图片域名".to_string())))
    }

    pub async fn display_image(
        &self,
        chapter_id: &str,
        filename: &str,
    ) -> Result<(Vec<u8>, &'static str), JmError> {
        let bytes = self.image_bytes(chapter_id, filename).await?;
        let scramble_id = self.scramble_id(chapter_id).await;
        let parsed_chapter = chapter_id.parse::<i64>().unwrap_or(0);
        let filename = filename.to_string();
        // 解码 + 解扰 + 编码是 CPU 密集操作，放到阻塞线程池，避免卡住异步运行时。
        tokio::task::spawn_blocking(move || {
            scramble::process_for_display(scramble_id, parsed_chapter, &filename, &bytes)
        })
        .await
        .map_err(|error| JmError::Protocol(format!("图片处理线程失败: {error}")))?
        .map_err(JmError::Protocol)
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    /// 手动冒烟测试：需要网络和代理。
    /// `JM_PROXY=http://127.0.0.1:7897 cargo test --lib live_jm_smoke -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn live_jm_smoke() {
        let client = JmClient::new();
        if let Ok(proxy) = std::env::var("JM_PROXY") {
            client.set_proxy(Some(proxy)).await.unwrap();
        }
        client.bootstrap().await.unwrap();
        let search = client
            .search("Yan-Yam", 1, "mr", 0, "a", None, None)
            .await
            .unwrap();
        assert!(search.total > 0, "搜索应有结果");
        let promote = client.promote().await.unwrap();
        assert!(!promote.is_empty(), "推荐位应有分区");
        let categories = client.categories().await.unwrap();
        assert!(!categories.categories.is_empty(), "分类列表应有内容");
        let weekly = client.weekly_info().await.unwrap();
        assert!(!weekly.categories.is_empty(), "周刊索引应有内容");
        let weekly_list = client
            .weekly_list(1, &weekly.categories[0].id, "manga")
            .await
            .unwrap();
        let ranking = client
            .category_comics(1, "0", "mv_t", "a")
            .await
            .unwrap();
        let latest = client.category_comics(1, "0", "mr", "a").await.unwrap();
        assert!(!latest.content.is_empty(), "最新列表应有内容");
        let tag_search = client
            .search("巨乳", 1, "mr", 3, "a", None, None)
            .await
            .unwrap();
        assert!(tag_search.total > 0, "标签搜索应有结果");
        let forum = client.forum("1485105", 1).await.unwrap();
        assert!(forum.1 > 0, "评论总数应大于 0");
        let daily = client.daily("1").await.unwrap();
        assert!(!daily.daily_id.is_empty(), "签到信息应有 daily_id");
        let bundle = client.comic_bundle("1485105").await.unwrap();
        assert_eq!(bundle.episodes.len(), 1);
        let chapter = client.chapter("1485105").await.unwrap();
        assert!(!chapter.images.is_empty(), "章节应有图片");
        let scramble = client.scramble_id("1485105").await;
        assert!(scramble > 0, "scramble_id 应大于 0");
        let (raw, content_type) = client
            .display_image("1485105", &chapter.images[0])
            .await
            .unwrap();
        assert!(raw.len() > 1000, "显示图片应有内容");
        let original = client
            .image_bytes("1485105", &chapter.images[0])
            .await
            .unwrap();
        let (jpeg, extension) =
            scramble::process_for_download(scramble, 1_485_105, &chapter.images[0], &original, "jpg")
                .unwrap();
        assert_eq!(extension, "jpg");
        assert!(jpeg.len() > 1000, "下载图片应有内容");
        println!(
            "search_total={} promote={} categories={} weekly_issues={} weekly_items={} ranking={} latest={} tag_total={} comments={} daily_id={} title={} images={} scramble={} content_type={} display_bytes={} jpeg_bytes={}",
            search.total,
            promote.len(),
            categories.categories.len(),
            weekly.categories.len(),
            weekly_list.list.len(),
            ranking.total,
            latest.content.len(),
            tag_search.total,
            forum.1,
            daily.daily_id,
            bundle.comic.title,
            chapter.images.len(),
            scramble,
            content_type,
            raw.len(),
            jpeg.len()
        );
    }
}
