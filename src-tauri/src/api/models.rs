use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Flexible deserializers: the JM API mixes numbers, numeric strings and nulls.
// ---------------------------------------------------------------------------

pub fn de_i64_flexible<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Number(n) => Ok(n.as_i64().unwrap_or(0)),
        serde_json::Value::String(s) => Ok(s.trim().parse::<i64>().unwrap_or(0)),
        serde_json::Value::Null => Ok(0),
        other => Err(D::Error::custom(format!(
            "expected number or numeric string, got {other:?}"
        ))),
    }
}

pub fn de_opt_i64_flexible<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Number(n) => Ok(n.as_i64()),
        serde_json::Value::String(s) => {
            if s.trim().is_empty() {
                Ok(None)
            } else {
                Ok(s.trim().parse::<i64>().ok())
            }
        }
        serde_json::Value::Null => Ok(None),
        other => Err(D::Error::custom(format!(
            "expected number or numeric string, got {other:?}"
        ))),
    }
}

pub fn de_string_flexible<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        serde_json::Value::Bool(b) => Ok(b.to_string()),
        serde_json::Value::Null => Ok(String::new()),
        other => Err(D::Error::custom(format!(
            "expected string-like value, got {other:?}"
        ))),
    }
}

pub fn de_bool_flexible<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_i64().unwrap_or(0) != 0),
        serde_json::Value::String(s) => Ok(matches!(s.as_str(), "1" | "true" | "True")),
        serde_json::Value::Null => Ok(false),
        other => Err(D::Error::custom(format!(
            "expected bool-like value, got {other:?}"
        ))),
    }
}

pub fn de_opt_bool_flexible<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::Bool(b) => Ok(Some(b)),
        serde_json::Value::Number(n) => Ok(Some(n.as_i64().unwrap_or(0) != 0)),
        serde_json::Value::String(s) => match s.as_str() {
            "" | "null" => Ok(None),
            "1" | "true" | "True" => Ok(Some(true)),
            _ => Ok(Some(false)),
        },
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmEnvelope {
    pub code: i64,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default, alias = "errorMsg")]
    pub error_msg: String,
}

// ---------------------------------------------------------------------------
// Raw JM entities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmSeries {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmRelated {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub author: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmCategory {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub title: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmAlbumSummary {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub author: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub description: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub name: String,
    #[serde(default)]
    pub category: Option<JmCategory>,
    #[serde(default, rename = "category_sub")]
    pub category_sub: Option<JmCategory>,
    #[serde(default, deserialize_with = "de_bool_flexible")]
    pub liked: bool,
    #[serde(default, rename = "is_favorite", deserialize_with = "de_bool_flexible")]
    pub is_favorite: bool,
    #[serde(default, deserialize_with = "de_i64_flexible")]
    pub update_at: i64,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub adddate: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmAlbum {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub name: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub addtime: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub description: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub total_views: String,
    #[serde(default, deserialize_with = "de_opt_i64_flexible")]
    pub total_photos: Option<i64>,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub likes: String,
    #[serde(default)]
    pub series: Vec<JmSeries>,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub comment_total: String,
    #[serde(default)]
    pub author: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub works: Vec<String>,
    #[serde(default)]
    pub actors: Vec<String>,
    #[serde(default, rename = "related_list")]
    pub related_list: Vec<JmRelated>,
    #[serde(default, deserialize_with = "de_bool_flexible")]
    pub liked: bool,
    #[serde(default, rename = "is_favorite", deserialize_with = "de_bool_flexible")]
    pub is_favorite: bool,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub price: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub purchased: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmChapter {
    #[serde(default)]
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmSearchResp {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub search_query: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub search_type: String,
    #[serde(default, deserialize_with = "de_i64_flexible")]
    pub total: i64,
    #[serde(default)]
    pub content: Vec<JmAlbumSummary>,
    #[serde(default, deserialize_with = "de_opt_i64_flexible")]
    pub redirect_aid: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmCategoriesResp {
    #[serde(default)]
    pub categories: Vec<JmApiCategory>,
    #[serde(default)]
    pub blocks: serde_json::Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmApiCategory {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub name: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub slug: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub total_albums: String,
    #[serde(default)]
    pub sub_categories: Vec<JmApiSubCategory>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmApiSubCategory {
    #[serde(default, rename = "CID", deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub name: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub slug: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmWeekInfo {
    #[serde(default)]
    pub categories: Vec<JmWeekCategory>,
    #[serde(default, rename = "type")]
    pub types: Vec<JmWeekType>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmWeekCategory {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub title: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub time: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmWeekType {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub title: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmWeekList {
    #[serde(default, deserialize_with = "de_i64_flexible")]
    pub total: i64,
    #[serde(default)]
    pub list: Vec<JmAlbumSummary>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmFavoriteResp {
    #[serde(default)]
    pub list: Vec<JmAlbumSummary>,
    #[serde(default, rename = "folder_list")]
    pub folder_list: Vec<JmFavoriteFolder>,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub total: String,
    #[serde(default, deserialize_with = "de_i64_flexible")]
    pub count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmFavoriteFolder {
    #[serde(default, rename = "FID", deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, rename = "UID", deserialize_with = "de_string_flexible")]
    pub owner_user_id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmForumResp {
    #[serde(default)]
    pub list: Vec<JmForumItem>,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub total: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmForumItem {
    #[serde(default, rename = "CID", deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub content: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub likes: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub username: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub nickname: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub photo: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub addtime: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub spoiler: String,
    #[serde(default)]
    pub expinfo: Option<JmExpInfo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmExpInfo {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub uid: String,
    #[serde(default, deserialize_with = "de_i64_flexible")]
    pub level: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmPromoteSection {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub title: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub slug: String,
    #[serde(default, rename = "type", deserialize_with = "de_string_flexible")]
    pub section_type: String,
    #[serde(default, rename = "filter_val", deserialize_with = "de_string_flexible")]
    pub filter_value: String,
    #[serde(default)]
    pub content: Vec<JmAlbumSummary>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmDailyRecord {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub date: String,
    #[serde(default, deserialize_with = "de_opt_bool_flexible")]
    pub signed: Option<bool>,
    #[serde(default, deserialize_with = "de_bool_flexible")]
    pub bonus: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JmDailyInfo {
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub daily_id: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub event_name: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub current_progress: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub three_days_coin: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub three_days_exp: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub seven_days_coin: String,
    #[serde(default, deserialize_with = "de_string_flexible")]
    pub seven_days_exp: String,
    #[serde(default)]
    pub record: Vec<Vec<JmDailyRecord>>,
}

// ---------------------------------------------------------------------------
// IPC-facing models (serialized to the Vue frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumb {
    pub path: String,
    pub file_server: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comic {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub author: String,
    pub pages_count: i64,
    pub eps_count: i64,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub works: Vec<String>,
    pub actors: Vec<String>,
    pub description: String,
    pub thumb: Option<Thumb>,
    pub likes_count: i64,
    pub views_count: i64,
    pub comment_count: i64,
    pub is_favourite: bool,
    pub is_liked: bool,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub related_list: Vec<Comic>,
    pub price: String,
    pub purchased: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub order: i64,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Picture {
    #[serde(rename = "_id")]
    pub id: String,
    pub filename: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PicturePage {
    pub docs: Vec<Picture>,
    pub total: i64,
    pub pages: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForumPage {
    pub docs: Vec<CommentItem>,
    pub total: i64,
    pub pages: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentUser {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub level: i64,
    pub avatar: Option<Thumb>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentItem {
    #[serde(rename = "_id")]
    pub id: String,
    pub content: String,
    pub likes_count: i64,
    pub is_spoiler: bool,
    pub created_at: String,
    pub user: CommentUser,
    pub replies: Vec<CommentItem>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUser {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub level: i64,
    pub level_name: String,
    pub exp: i64,
    pub exp_percent: f64,
    pub coin: i64,
    pub avatar: Option<Thumb>,
    pub favourite_count: i64,
    pub max_favourite_count: i64,
    pub is_punched: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub slug: String,
    pub total_albums: i64,
    pub sub_categories: Vec<SubCategory>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubCategory {
    #[serde(rename = "_id")]
    pub id: String,
    pub title: String,
    pub slug: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoriesResult {
    pub categories: Vec<Category>,
    pub blocks: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComicBundle {
    pub comic: Comic,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekCategory {
    pub id: String,
    pub title: String,
    pub time: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekType {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyInfo {
    pub categories: Vec<WeekCategory>,
    pub types: Vec<WeekType>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekList {
    pub total: i64,
    pub list: Vec<Comic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub search_query: String,
    pub search_type: String,
    pub total: i64,
    pub content: Vec<Comic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_album: Option<Box<Comic>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteFolder {
    #[serde(rename = "_id")]
    pub id: String,
    pub name: String,
    pub owner_user_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteResult {
    pub list: Vec<Comic>,
    pub folders: Vec<FavoriteFolder>,
    pub total: i64,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoteSection {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub section_type: String,
    pub filter_value: String,
    pub content: Vec<Comic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoteList {
    pub total: i64,
    pub list: Vec<Comic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchListResult {
    pub total: i64,
    pub list: Vec<Comic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRecord {
    pub date: String,
    pub signed: Option<bool>,
    pub bonus: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyInfo {
    pub daily_id: String,
    pub event_name: String,
    pub current_progress: String,
    pub three_days_coin: String,
    pub three_days_exp: String,
    pub seven_days_coin: String,
    pub seven_days_exp: String,
    pub records: Vec<DailyRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub status: String,
    pub message: String,
    pub action_type: String,
}

// Download tracking (IPC-facing)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadTask {
    pub id: String,
    pub comic_id: String,
    pub comic_title: String,
    pub chapter_id: String,
    pub episode_order: i64,
    pub episode_title: String,
    /// pending | downloading | paused | completed | failed | cancelled
    pub status: String,
    pub total_images: i64,
    pub downloaded_images: i64,
    pub save_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalComic {
    pub comic_id: String,
    pub title: String,
    pub author: Option<String>,
    pub pages_count: Option<i64>,
    pub category: Option<String>,
    pub eps_count: Option<i64>,
    pub update_time: Option<String>,
    pub downloaded_episodes: Option<Vec<String>>,
    pub cover_path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadHistoryEntry {
    pub comic_id: String,
    pub title: String,
    pub chapter_ids: Vec<String>,
    #[serde(default)]
    pub first_downloaded_at: Option<String>,
    #[serde(default)]
    pub last_downloaded_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadHistoryExport {
    pub version: u32,
    pub exported_at: String,
    pub comics: Vec<DownloadHistoryEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_comics: i64,
    pub total_episodes: i64,
    pub total_images: i64,
    pub total_size_bytes: i64,
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

pub fn normalize_host(host: &str) -> String {
    let host = host.trim().trim_end_matches('/');
    if host.starts_with("http://") || host.starts_with("https://") {
        host.to_string()
    } else {
        format!("https://{host}")
    }
}

pub fn cover_url(img_host: &str, album_id: &str, variant: &str) -> String {
    let host = normalize_host(img_host);
    let variant = variant.trim_matches('_');
    let variant = if variant.is_empty() { "3x4" } else { variant };
    format!("{host}/media/albums/{album_id}_{variant}.jpg")
}

pub fn photo_url(img_host: &str, chapter_id: &str, filename: &str) -> String {
    let host = normalize_host(img_host);
    format!(
        "{host}/media/photos/{chapter_id}/{}",
        filename.trim_start_matches('/')
    )
}

pub fn unix_to_date(value: &str) -> String {
    value
        .trim()
        .parse::<i64>()
        .ok()
        .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

pub fn unix_to_datetime(value: i64) -> String {
    chrono::DateTime::from_timestamp(value, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

fn parse_count(value: &str) -> i64 {
    value.trim().parse::<i64>().unwrap_or(0)
}

pub fn comic_from_summary(item: JmAlbumSummary, img_host: &str) -> Comic {
    let mut categories = Vec::new();
    // category 与 category_sub 常常是同一个名字，去重后再展示。
    for category in [item.category.as_ref(), item.category_sub.as_ref()]
        .into_iter()
        .flatten()
    {
        let title = category.title.trim();
        if !title.is_empty() && !categories.iter().any(|existing| existing == title) {
            categories.push(title.to_string());
        }
    }
    let thumb = if item.id.is_empty() {
        None
    } else {
        Some(Thumb {
            path: cover_url(img_host, &item.id, "3x4"),
            file_server: String::new(),
        })
    };
    Comic {
        id: item.id,
        title: item.name,
        author: item.author,
        pages_count: 0,
        eps_count: 0,
        categories,
        tags: Vec::new(),
        works: Vec::new(),
        actors: Vec::new(),
        description: item.description,
        thumb,
        likes_count: 0,
        views_count: 0,
        comment_count: 0,
        is_favourite: item.is_favorite,
        is_liked: item.liked,
        updated_at: unix_to_datetime(item.update_at),
        created_at: item.adddate,
        related_list: Vec::new(),
        price: String::new(),
        purchased: String::new(),
    }
}

pub fn comic_from_album(album: JmAlbum, img_host: &str) -> Comic {
    let thumb = Some(Thumb {
        path: cover_url(img_host, &album.id, "3x4"),
        file_server: String::new(),
    });
    let related_list = album
        .related_list
        .into_iter()
        .map(|item| {
            let thumb = Some(Thumb {
                path: cover_url(img_host, &item.id, "3x4"),
                file_server: String::new(),
            });
            Comic {
            id: item.id,
            title: item.name,
            author: item.author,
            pages_count: 0,
            eps_count: 0,
            categories: Vec::new(),
            tags: Vec::new(),
            works: Vec::new(),
            actors: Vec::new(),
            description: String::new(),
            thumb,
            likes_count: 0,
            views_count: 0,
            comment_count: 0,
            is_favourite: false,
            is_liked: false,
            updated_at: String::new(),
            created_at: String::new(),
            related_list: Vec::new(),
            price: String::new(),
            purchased: String::new(),
        }})
        .collect();
    Comic {
        id: album.id,
        title: album.name,
        author: album.author.join(", "),
        pages_count: album.total_photos.unwrap_or(0),
        eps_count: album.series.len().max(1) as i64,
        // /album 不返回 category，站点分类来自列表接口；这里只保留内容标签。
        categories: Vec::new(),
        tags: album.tags,
        works: album.works,
        actors: album.actors,
        description: album.description,
        thumb,
        likes_count: parse_count(&album.likes),
        views_count: parse_count(&album.total_views),
        comment_count: parse_count(&album.comment_total),
        is_favourite: album.is_favorite,
        is_liked: album.liked,
        updated_at: unix_to_date(&album.addtime),
        created_at: unix_to_date(&album.addtime),
        related_list,
        price: album.price,
        purchased: album.purchased,
    }
}

pub fn episodes_from_album(album: &JmAlbum) -> Vec<Episode> {
    if album.series.is_empty() {
        return vec![Episode {
            id: album.id.clone(),
            title: "第1话".to_string(),
            order: 1,
            updated_at: None,
        }];
    }
    album
        .series
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let order = (index + 1) as i64;
            let title = if item.name.trim().is_empty() {
                format!("第{order}话")
            } else {
                format!("第{order}话 {}", item.name.trim())
            };
            Episode {
                id: item.id.clone(),
                title,
                order,
                updated_at: None,
            }
        })
        .collect()
}

pub fn comments_from_forum(resp: JmForumResp, img_host: &str) -> (Vec<CommentItem>, i64) {
    let total = parse_count(&resp.total);
    let mut items = Vec::new();
    for item in resp.list {
        let user_name = if !item.nickname.trim().is_empty() {
            item.nickname
        } else {
            item.username
        };
        let level = item.expinfo.as_ref().map(|info| info.level).unwrap_or(0);
        let avatar = if item.photo.trim().is_empty() || item.photo == "nopic-Male.gif" {
            None
        } else if item.photo.starts_with("http") {
            Some(Thumb {
                path: item.photo.clone(),
                file_server: String::new(),
            })
        } else {
            Some(Thumb {
                path: format!("{}/media/users/{}", normalize_host(img_host), item.photo),
                file_server: String::new(),
            })
        };
        items.push(CommentItem {
            id: item.id,
            content: item.content,
            likes_count: parse_count(&item.likes),
            is_spoiler: item.spoiler == "1" || item.spoiler == "true",
            created_at: item.addtime,
            user: CommentUser {
                id: item
                    .expinfo
                    .as_ref()
                    .map(|info| info.uid.clone())
                    .unwrap_or_default(),
                name: user_name,
                level,
                avatar,
            },
            replies: Vec::new(),
        });
    }
    (items, total)
}
