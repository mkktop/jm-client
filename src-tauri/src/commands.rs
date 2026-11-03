use crate::api::{
    ActionResult, CategoriesResult, Comic, ComicBundle, CommentItem, DailyInfo, DownloadHistoryEntry,
    DownloadHistoryExport, DownloadTask, Episode, FavoriteResult, ForumPage, JmClient, Picture,
    PicturePage, ProfileUser, PromoteList, PromoteSection, SearchResult, WatchListResult, WeekList,
    WeeklyInfo,
};
use crate::db::Database;
use crate::download::DownloadManager;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::Manager;
use tauri::State;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageCacheConfig {
    pub enabled: bool,
    pub limit_bytes: u64,
}

pub struct AppState {
    pub client: Arc<JmClient>,
    pub db: Arc<Database>,
    pub download_manager: Arc<DownloadManager>,
    pub download_dir: Arc<RwLock<String>>,
    pub image_cache: Arc<RwLock<ImageCacheConfig>>,
    pub image_cache_used: Arc<AtomicU64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub keyword: String,
    pub page: i64,
    pub sort: String,
    #[serde(default)]
    pub main_tag: i64,
    #[serde(default)]
    pub time: String,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub month: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadRequest {
    pub comic_id: String,
    pub comic_title: String,
    pub chapter_id: String,
    pub chapter_title: String,
    pub episode_order: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentRequest {
    pub comic_id: String,
    pub content: String,
    #[serde(default)]
    pub comment_id: Option<String>,
    #[serde(default)]
    pub spoiler: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSettings {
    pub format: String,
    pub concurrency: usize,
    pub interval_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ImageCacheStatus {
    pub enabled: bool,
    pub limit_mb: u64,
    pub used_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct DownloadStateSets {
    pub history: Vec<String>,
    pub local: Vec<String>,
}

async fn clear_persisted_session(state: &AppState) {
    state.client.clear_session().await;
    let _ = state.db.set_setting("jm_avs", "").await;
    let _ = state.db.set_setting("jm_login_domain", "").await;
    let _ = state.db.set_setting("jm_user_id", "").await;
}

async fn persist_session(state: &AppState) {
    let avs = state.client.avs().await.unwrap_or_default();
    let domain = state.client.login_domain().await.unwrap_or_default();
    let user_id = state.client.user_id().await.unwrap_or_default();
    let _ = state.db.set_setting("jm_avs", &avs).await;
    let _ = state.db.set_setting("jm_login_domain", &domain).await;
    let _ = state.db.set_setting("jm_user_id", &user_id).await;
}

#[tauri::command]
pub async fn login(state: State<'_, AppState>, req: LoginRequest) -> Result<ProfileUser, String> {
    let profile = state
        .client
        .login(&req.username, &req.password)
        .await
        .map_err(|error| error.to_string())?;
    persist_session(state.inner()).await;
    Ok(profile)
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    state.client.logout().await;
    clear_persisted_session(state.inner()).await;
    Ok(())
}

#[tauri::command]
pub async fn is_logged_in(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.client.is_logged_in().await)
}

#[tauri::command]
pub async fn get_my_profile(state: State<'_, AppState>) -> Result<ProfileUser, String> {
    match state.client.get_profile().await {
        Ok(profile) => Ok(profile),
        Err(error) => {
            if matches!(error, crate::api::JmError::Auth) {
                clear_persisted_session(state.inner()).await;
            }
            Err(error.to_string())
        }
    }
}

#[tauri::command]
pub async fn search_comics(
    state: State<'_, AppState>,
    req: SearchRequest,
) -> Result<SearchResult, String> {
    let sort = match req.sort.as_str() {
        "mv" | "mp" | "tf" | "tr" | "md" => req.sort.as_str(),
        _ => "mr",
    };
    let time = match req.time.as_str() {
        "t" | "w" | "m" => req.time.as_str(),
        _ => "a",
    };
    let main_tag = if (0..=4).contains(&req.main_tag) {
        req.main_tag
    } else {
        0
    };
    state
        .client
        .search(
            &req.keyword,
            req.page.max(1),
            sort,
            main_tag,
            time,
            req.year,
            req.month,
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_categories(state: State<'_, AppState>) -> Result<CategoriesResult, String> {
    state
        .client
        .categories()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_category_comics(
    state: State<'_, AppState>,
    page: i64,
    category: String,
    order: String,
    time: String,
) -> Result<SearchResult, String> {
    let order = match order.as_str() {
        "mv" | "mp" | "tf" | "tr" | "md" => order.as_str(),
        _ => "mr",
    };
    let time = match time.as_str() {
        "t" | "w" | "m" => time.as_str(),
        _ => "a",
    };
    state
        .client
        .category_comics(page.max(1), category.trim(), order, time)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_ranking(
    state: State<'_, AppState>,
    page: i64,
    period: String,
    category: String,
) -> Result<SearchResult, String> {
    let period = match period.as_str() {
        "mv_w" | "mv_m" => period.as_str(),
        _ => "mv_t",
    };
    let category = if category.trim().is_empty() {
        "0"
    } else {
        category.trim()
    };
    state
        .client
        .category_comics(page.max(1), category, period, "a")
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_latest_comics(
    state: State<'_, AppState>,
    page: i64,
) -> Result<SearchResult, String> {
    state
        .client
        .category_comics(page.max(1), "0", "mr", "a")
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_weekly_info(state: State<'_, AppState>) -> Result<WeeklyInfo, String> {
    state
        .client
        .weekly_info()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_weekly(
    state: State<'_, AppState>,
    page: i64,
    category_id: String,
    type_id: String,
) -> Result<WeekList, String> {
    state
        .client
        .weekly_list(page.max(1), &category_id, &type_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_promote(state: State<'_, AppState>) -> Result<Vec<PromoteSection>, String> {
    state
        .client
        .promote()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_promote_list(
    state: State<'_, AppState>,
    section_id: String,
    page: i64,
) -> Result<PromoteList, String> {
    state
        .client
        .promote_list(&section_id, page.max(1))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_comic_info(state: State<'_, AppState>, comic_id: String) -> Result<Comic, String> {
    state
        .client
        .comic(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_comic_bundle(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<ComicBundle, String> {
    state
        .client
        .comic_bundle(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_episodes(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<Vec<Episode>, String> {
    state
        .client
        .episodes(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_pictures_page(
    state: State<'_, AppState>,
    comic_id: String,
    chapter_id: String,
    page: i64,
) -> Result<PicturePage, String> {
    let _ = comic_id;
    let _ = page;
    let chapter = state
        .client
        .chapter(&chapter_id)
        .await
        .map_err(|error| error.to_string())?;
    let docs: Vec<Picture> = chapter
        .images
        .into_iter()
        .filter(|filename| !filename.trim().is_empty())
        .map(|filename| Picture {
            id: filename.clone(),
            url: format!("jmimg://photo/{chapter_id}/{filename}"),
            filename,
            local_path: None,
        })
        .collect();
    let total = docs.len() as i64;
    Ok(PicturePage {
        docs,
        total,
        pages: 1,
    })
}

#[tauri::command]
pub async fn get_local_pictures(
    state: State<'_, AppState>,
    comic_id: String,
    chapter_id: String,
) -> Result<Option<Vec<String>>, String> {
    let Some(path) = state
        .db
        .get_episode_save_path(&comic_id, &chapter_id)
        .await
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    let dir = std::path::PathBuf::from(&path);
    if !dir.is_dir() {
        let _ = state
            .db
            .cleanup_missing_downloads_for_comic(&comic_id)
            .await;
        return Ok(None);
    }
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|error| error.to_string())?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            let ext = path.extension()?.to_str()?.to_ascii_lowercase();
            if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif" | "avif") {
                Some(path.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    if files.is_empty() {
        return Ok(None);
    }
    files.sort();
    Ok(Some(files))
}

#[tauri::command]
pub async fn get_forum(
    state: State<'_, AppState>,
    comic_id: String,
    page: i64,
) -> Result<ForumPage, String> {
    let (docs, total) = state
        .client
        .forum(&comic_id, page.max(1))
        .await
        .map_err(|error| error.to_string())?;
    let pages = ((total as f64) / 10.0).ceil() as i64;
    Ok(ForumPage {
        docs,
        total,
        pages: pages.max(1),
    })
}

#[tauri::command]
pub async fn post_comment(
    state: State<'_, AppState>,
    req: CommentRequest,
) -> Result<ActionResult, String> {
    state
        .client
        .post_comment(
            &req.comic_id,
            &req.content,
            req.comment_id.as_deref(),
            req.spoiler,
        )
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn toggle_like(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<ActionResult, String> {
    state
        .client
        .toggle_like(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn toggle_favourite(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<ActionResult, String> {
    state
        .client
        .toggle_favorite(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_my_favourites(
    state: State<'_, AppState>,
    page: i64,
    sort: String,
    folder_id: String,
) -> Result<FavoriteResult, String> {
    let sort = if sort == "mp" { "mp" } else { "mr" };
    let folder_id = if folder_id.trim().is_empty() {
        "0"
    } else {
        folder_id.trim()
    };
    state
        .client
        .favorite(page.max(1), sort, folder_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn move_favourite(
    state: State<'_, AppState>,
    comic_id: String,
    folder_id: String,
) -> Result<ActionResult, String> {
    state
        .client
        .move_favorite(&comic_id, &folder_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_watch_list(
    state: State<'_, AppState>,
    page: i64,
) -> Result<WatchListResult, String> {
    state
        .client
        .watch_list(page.max(1))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_daily(state: State<'_, AppState>) -> Result<DailyInfo, String> {
    let user_id = state
        .client
        .user_id()
        .await
        .ok_or_else(|| "请先登录".to_string())?;
    state
        .client
        .daily(&user_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn daily_checkin(state: State<'_, AppState>) -> Result<ActionResult, String> {
    let user_id = state
        .client
        .user_id()
        .await
        .ok_or_else(|| "请先登录".to_string())?;
    let daily = state
        .client
        .daily(&user_id)
        .await
        .map_err(|error| error.to_string())?;
    state
        .client
        .daily_checkin(&user_id, &daily.daily_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn start_download(
    state: State<'_, AppState>,
    req: DownloadRequest,
) -> Result<String, String> {
    let download_dir = state.download_dir.read().await.clone();
    let task_id = uuid::Uuid::new_v4().to_string();
    let task = DownloadTask {
        id: task_id.clone(),
        comic_id: req.comic_id.clone(),
        comic_title: req.comic_title.clone(),
        chapter_id: req.chapter_id.clone(),
        episode_order: req.episode_order,
        episode_title: req.chapter_title.clone(),
        status: "pending".into(),
        total_images: 0,
        downloaded_images: 0,
        save_path: download_dir.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    state
        .db
        .add_download_task(&task)
        .await
        .map_err(|error| error.to_string())?;
    state.download_manager.register(&task_id).await;

    let settings = read_download_settings(state.inner()).await;
    let dm = state.download_manager.clone();
    let client = state.client.clone();
    let db = state.db.clone();
    let task_id_for_task = task_id.clone();
    tokio::spawn(async move {
        dm.download_episode(
            client,
            db,
            task_id_for_task,
            req.comic_id,
            req.comic_title,
            req.chapter_id,
            req.chapter_title,
            req.episode_order,
            download_dir,
            settings.format,
            settings.concurrency,
            settings.interval_ms,
        )
        .await;
    });
    Ok(task_id)
}

#[tauri::command]
pub async fn pause_download(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    if state.download_manager.pause(&task_id).await {
        let _ = state.db.set_download_status(&task_id, "paused", None).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn resume_download(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    if state.download_manager.resume(&task_id).await {
        let _ = state
            .db
            .set_download_status(&task_id, "downloading", None)
            .await;
    }
    Ok(())
}

#[tauri::command]
pub async fn cancel_download(state: State<'_, AppState>, task_id: String) -> Result<(), String> {
    if state.download_manager.cancel(&task_id).await {
        let _ = state
            .db
            .set_download_status(&task_id, "cancelled", Some("用户取消"))
            .await;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_download_queue(state: State<'_, AppState>) -> Result<Vec<DownloadTask>, String> {
    state
        .db
        .get_all_downloads(200)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_download_history(
    state: State<'_, AppState>,
) -> Result<Vec<DownloadHistoryEntry>, String> {
    state
        .db
        .get_download_history()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_downloaded_chapter_ids(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<Vec<String>, String> {
    let _ = state
        .db
        .cleanup_missing_downloads_for_comic(&comic_id)
        .await;
    state
        .db
        .get_downloaded_chapter_ids(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_downloaded_episode_orders(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<Vec<i64>, String> {
    let _ = state
        .db
        .cleanup_missing_downloads_for_comic(&comic_id)
        .await;
    state
        .db
        .get_downloaded_episode_orders(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_history_chapter_ids(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<Vec<String>, String> {
    state
        .db
        .get_history_chapter_ids(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_history_episode_orders(
    state: State<'_, AppState>,
    comic_id: String,
) -> Result<Vec<i64>, String> {
    state
        .db
        .get_history_episode_orders(&comic_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_download_state_sets(
    state: State<'_, AppState>,
) -> Result<DownloadStateSets, String> {
    let (history, local) = state
        .db
        .get_download_state_sets()
        .await
        .map_err(|error| error.to_string())?;
    Ok(DownloadStateSets { history, local })
}

#[tauri::command]
pub async fn clear_finished_download_tasks(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .db
        .clear_finished_download_tasks()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn cleanup_library(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .db
        .cleanup_missing_downloads()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_library(
    state: State<'_, AppState>,
    limit: i64,
    offset: i64,
) -> Result<serde_json::Value, String> {
    state
        .db
        .get_comics(limit, offset)
        .await
        .map(|items| serde_json::to_value(items).unwrap_or_default())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_library_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    state
        .db
        .get_stats()
        .await
        .map(|stats| serde_json::to_value(stats).unwrap_or_default())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_download_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.download_dir.read().await.clone())
}

#[tauri::command]
pub async fn set_download_dir(state: State<'_, AppState>, dir: String) -> Result<(), String> {
    *state.download_dir.write().await = dir.clone();
    state
        .db
        .set_setting("download_dir", &dir)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn pick_download_dir(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        use tauri_plugin_dialog::DialogExt;
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|path| {
                path.into_path()
                    .map(|path| path.to_string_lossy().to_string())
                    .map_err(|error| error.to_string())
            })
            .transpose()
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn get_proxy(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.client.get_proxy().await)
}

#[tauri::command]
pub async fn set_proxy(state: State<'_, AppState>, proxy: String) -> Result<(), String> {
    let value = proxy.trim().to_string();
    state
        .client
        .set_proxy(if value.is_empty() {
            None
        } else {
            Some(value.clone())
        })
        .await
        .map_err(|error| error.to_string())?;
    state
        .db
        .set_setting("proxy_url", &value)
        .await
        .map_err(|error| error.to_string())?;
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        state.client.bootstrap(),
    )
    .await;
    let host = state.client.img_host().await;
    let _ = state.db.set_setting("img_host", &host).await;
    Ok(())
}

#[tauri::command]
pub async fn get_api_domain(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.client.api_domain().await)
}

#[tauri::command]
pub async fn get_api_domains(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(state.client.api_domains().await)
}

#[tauri::command]
pub async fn get_api_version(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.client.api_version().await)
}

#[tauri::command]
pub async fn set_api_domain(
    state: State<'_, AppState>,
    domain: String,
) -> Result<(), String> {
    state
        .client
        .set_api_domain(&domain)
        .await
        .map_err(|error| error.to_string())?;
    state
        .db
        .set_setting("api_domain", domain.trim())
        .await
        .map_err(|error| error.to_string())?;
    if !state.client.is_logged_in().await {
        clear_persisted_session(state.inner()).await;
    }
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        state.client.bootstrap(),
    )
    .await;
    let host = state.client.img_host().await;
    let _ = state.db.set_setting("img_host", &host).await;
    Ok(())
}

#[tauri::command]
pub async fn refresh_api_domains(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .client
        .refresh_api_domains()
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_img_host(state: State<'_, AppState>) -> Result<String, String> {
    Ok(state.client.img_host().await)
}

#[tauri::command]
pub async fn set_img_host(state: State<'_, AppState>, host: String) -> Result<(), String> {
    let value = host.trim().to_string();
    if value.is_empty() {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(12),
            state.client.bootstrap(),
        )
        .await;
    } else {
        state.client.set_img_host(value).await;
    }
    let actual = state.client.img_host().await;
    state
        .db
        .set_setting("img_host", &actual)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_image_cache_status(
    state: State<'_, AppState>,
) -> Result<ImageCacheStatus, String> {
    let config = state.image_cache.read().await.clone();
    Ok(ImageCacheStatus {
        enabled: config.enabled,
        limit_mb: config.limit_bytes / (1024 * 1024),
        used_bytes: state.image_cache_used.load(Ordering::Relaxed),
    })
}

#[tauri::command]
pub async fn set_image_cache_settings(
    state: State<'_, AppState>,
    enabled: bool,
    limit_mb: u64,
) -> Result<(), String> {
    let limit_mb = limit_mb.clamp(16, 8192);
    state
        .db
        .set_setting("image_cache_enabled", if enabled { "1" } else { "0" })
        .await
        .map_err(|error| error.to_string())?;
    state
        .db
        .set_setting("image_cache_limit_mb", &limit_mb.to_string())
        .await
        .map_err(|error| error.to_string())?;
    *state.image_cache.write().await = ImageCacheConfig {
        enabled,
        limit_bytes: limit_mb * 1024 * 1024,
    };
    Ok(())
}

#[tauri::command]
pub async fn clear_image_cache(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?
        .join("imgcache");
    let freed = crate::dir_used_bytes(&dir);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|error| error.to_string())?;
    }
    state.image_cache_used.store(0, Ordering::Relaxed);
    Ok(freed)
}

/// 清空本地数据与缓存，并重启应用，回到全新安装状态。
/// 下载目录中已保存的漫画文件由用户自行管理，不会删除。
#[tauri::command]
pub async fn clear_all_data(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .db
        .wipe_all()
        .await
        .map_err(|error| format!("清除本地数据失败：{error}"))?;
    if let Ok(cache_dir) = app.path().app_cache_dir() {
        // 封面缓存与更新安装包都可再生成，直接清空。
        let _ = std::fs::remove_dir_all(cache_dir.join("imgcache"));
        let _ = std::fs::remove_dir_all(cache_dir.join("updates"));
    }
    state.image_cache_used.store(0, Ordering::Relaxed);
    app.restart();
}

async fn read_download_settings(state: &AppState) -> DownloadSettings {
    let format = state
        .db
        .get_setting("download_format")
        .await
        .ok()
        .flatten()
        .filter(|value| matches!(value.as_str(), "jpg" | "jpeg" | "png" | "webp"))
        .unwrap_or_else(|| "jpg".to_string());
    let concurrency = state
        .db
        .get_setting("download_concurrency")
        .await
        .ok()
        .flatten()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(8)
        .clamp(1, 32);
    let interval_ms = state
        .db
        .get_setting("download_interval_ms")
        .await
        .ok()
        .flatten()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(300)
        .min(5000);
    DownloadSettings {
        format,
        concurrency,
        interval_ms,
    }
}

#[tauri::command]
pub async fn get_download_settings(state: State<'_, AppState>) -> Result<DownloadSettings, String> {
    Ok(read_download_settings(state.inner()).await)
}

#[tauri::command]
pub async fn save_download_settings(
    state: State<'_, AppState>,
    settings: DownloadSettings,
) -> Result<(), String> {
    let format = match settings.format.as_str() {
        "png" | "webp" => settings.format.as_str(),
        _ => "jpg",
    };
    let concurrency = settings.concurrency.clamp(1, 32);
    let interval = settings.interval_ms.min(5000);
    state
        .db
        .set_setting("download_format", format)
        .await
        .map_err(|error| error.to_string())?;
    state
        .db
        .set_setting("download_concurrency", &concurrency.to_string())
        .await
        .map_err(|error| error.to_string())?;
    state
        .db
        .set_setting("download_interval_ms", &interval.to_string())
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    state
        .db
        .get_setting(&key)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn save_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    state
        .db
        .set_setting(&key, &value)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn export_download_history(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let entries = state
        .db
        .get_download_history()
        .await
        .map_err(|error| error.to_string())?;
    let export = DownloadHistoryExport {
        version: 2,
        exported_at: chrono::Utc::now().to_rfc3339(),
        comics: entries,
    };
    let payload = serde_json::to_vec_pretty(&export).map_err(|error| error.to_string())?;
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name("jm-download-history.json")
            .blocking_save_file()
    })
    .await
    .map_err(|error| error.to_string())?;
    let Some(path) = file else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|error| error.to_string())?
        .with_extension("json");
    std::fs::write(&path, payload).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn import_download_history(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let file = tauri::async_runtime::spawn_blocking(move || {
        app.dialog().file().blocking_pick_file()
    })
    .await
    .map_err(|error| error.to_string())?;
    let Some(path) = file else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|error| error.to_string())?;
    let raw = std::fs::read_to_string(&path).map_err(|error| format!("读取文件失败：{error}"))?;
    let import: DownloadHistoryExport =
        serde_json::from_str(&raw).map_err(|error| format!("文件格式不正确：{error}"))?;
    if import.version != 2 {
        return Err(format!("不支持的记录版本：{}", import.version));
    }
    let comics: Vec<DownloadHistoryEntry> = import
        .comics
        .into_iter()
        .filter(|entry| !entry.comic_id.trim().is_empty() && !entry.title.trim().is_empty())
        .collect();
    let count = comics.len();
    state
        .db
        .merge_download_history(&comics)
        .await
        .map_err(|error| error.to_string())?;
    Ok(Some(format!("已导入 {count} 部漫画的下载记录")))
}

// Keep CommentItem referenced for command signatures generated by specta/tauri.
#[allow(dead_code)]
fn _comment_type_marker(_: CommentItem) {}
