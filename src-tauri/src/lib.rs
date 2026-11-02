mod api;
mod commands;
mod db;
mod download;
mod updates;

use commands::*;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

/// jmimg:// 自定义协议：封面/头像/章节图统一走 Rust 侧。
/// - jmimg://remote/<编码后的远程URL>：封面/头像，缓存于 app_cache_dir/imgcache。
/// - jmimg://local/<编码后的绝对路径>：离线阅读，仅允许读取下载目录内文件。
/// - jmimg://photo/<chapter_id>/<filename>：JM 章节图，下载后按 scramble_id 还原。
fn handle_jmimg(
    app: tauri::AppHandle,
    request: tauri::http::Request<Vec<u8>>,
    responder: tauri::UriSchemeResponder,
) {
    fn respond(status: u16, content_type: &str, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
        tauri::http::Response::builder()
            .status(status)
            .header("Content-Type", content_type)
            .body(body)
            .unwrap_or_else(|_| tauri::http::Response::new(Vec::new()))
    }
    fn content_type_of(path: &str) -> &'static str {
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "avif" => "image/avif",
            _ => "image/jpeg",
        }
    }
    fn guess_image_content_type(bytes: &[u8]) -> &'static str {
        if bytes.starts_with(b"GIF8") {
            "image/gif"
        } else if bytes.starts_with(b"\x89PNG") {
            "image/png"
        } else if bytes.starts_with(&[0xff, 0xd8]) {
            "image/jpeg"
        } else {
            "image/webp"
        }
    }

    let uri = request.uri().to_string();
    log::debug!("jmimg request: {uri}");
    let rest = uri
        .strip_prefix("http://jmimg.localhost/")
        .or_else(|| uri.strip_prefix("https://jmimg.localhost/"))
        .or_else(|| uri.strip_prefix("jmimg://localhost/"))
        .or_else(|| uri.strip_prefix("jmimg://"))
        .map(|value| value.to_string());
    let Some(rest) = rest else {
        log::warn!("jmimg rejected unexpected uri shape: {uri}");
        responder.respond(respond(400, "text/plain", b"bad scheme".to_vec()));
        return;
    };
    let (kind, encoded) = match rest.split_once('/') {
        Some((kind, value)) => (kind.to_ascii_lowercase(), value.trim_start_matches('/')),
        None => {
            responder.respond(respond(400, "text/plain", b"bad request".to_vec()));
            return;
        }
    };
    let decoded = match urlencoding::decode(encoded) {
        Ok(value) => value.into_owned(),
        Err(_) => {
            responder.respond(respond(400, "text/plain", b"bad encoding".to_vec()));
            return;
        }
    };

    match kind.as_str() {
        "local" => {
            let dir = app
                .try_state::<AppState>()
                .map(|state| {
                    tauri::async_runtime::block_on(async { state.download_dir.read().await.clone() })
                })
                .unwrap_or_default();
            let base = std::fs::canonicalize(&dir).unwrap_or_else(|_| std::path::PathBuf::from(&dir));
            let path = std::path::PathBuf::from(&decoded);
            let real = match std::fs::canonicalize(&path) {
                Ok(path) => path,
                Err(_) => {
                    responder.respond(respond(404, "text/plain", b"not found".to_vec()));
                    return;
                }
            };
            if !real.starts_with(&base) {
                responder.respond(respond(403, "text/plain", b"forbidden".to_vec()));
                return;
            }
            match std::fs::read(&real) {
                Ok(bytes) => responder.respond(respond(200, content_type_of(&decoded), bytes)),
                Err(_) => responder.respond(respond(404, "text/plain", b"read failed".to_vec())),
            }
        }
        "remote" => {
            if !decoded.starts_with("http://") && !decoded.starts_with("https://") {
                responder.respond(respond(400, "text/plain", b"bad url".to_vec()));
                return;
            }
            let content_type = content_type_of(&decoded);
            let config = app
                .try_state::<AppState>()
                .map(|state| {
                    tauri::async_runtime::block_on(async { state.image_cache.read().await.clone() })
                })
                .unwrap_or(ImageCacheConfig {
                    enabled: false,
                    limit_bytes: 0,
                });
            let cache_root = app
                .path()
                .app_cache_dir()
                .unwrap_or_default()
                .join("imgcache");
            let hex = hex_prefix(&decoded);
            let cache_file = cache_root.join(&hex[..2]).join(&hex);
            if config.enabled {
                if let Ok(bytes) = std::fs::read(&cache_file) {
                    responder.respond(respond(200, content_type, bytes));
                    return;
                }
            }
            tauri::async_runtime::spawn(async move {
                let Ok(_permit) = image_download_limit().acquire().await else {
                    responder.respond(respond(503, "text/plain", b"busy".to_vec()));
                    return;
                };
                let Some(state) = app.try_state::<AppState>() else {
                    responder.respond(respond(500, "text/plain", b"state unavailable".to_vec()));
                    return;
                };
                let client = state.client.clone();
                let used_counter = state.image_cache_used.clone();
                match client.image_bytes_direct(&decoded).await {
                    Ok(bytes) => {
                        if config.enabled && bytes.len() <= 8 * 1024 * 1024 {
                            let dir_ready = cache_file
                                .parent()
                                .map(|parent| std::fs::create_dir_all(parent).is_ok())
                                .unwrap_or(false);
                            let written = dir_ready && std::fs::write(&cache_file, &bytes).is_ok();
                            if written {
                                let size = bytes.len() as u64;
                                let used = used_counter
                                    .fetch_add(size, std::sync::atomic::Ordering::Relaxed)
                                    + size;
                                if config.limit_bytes > 0 && used > config.limit_bytes {
                                    evict_cache(
                                        &cache_root,
                                        &used_counter,
                                        config.limit_bytes / 10 * 9,
                                    );
                                }
                            }
                        }
                        responder.respond(respond(200, content_type, bytes));
                    }
                    Err(error) => {
                        log::warn!("jmimg fetch failed for {decoded}: {error}");
                        responder.respond(respond(502, "text/plain", b"fetch failed".to_vec()));
                    }
                }
            });
        }
        "photo" => {
            let Some((chapter_id, filename)) = decoded.split_once('/') else {
                responder.respond(respond(400, "text/plain", b"bad photo path".to_vec()));
                return;
            };
            if chapter_id.is_empty()
                || filename.is_empty()
                || !chapter_id.chars().all(|c| c.is_ascii_digit())
                || filename.contains('/')
                || filename.contains("..")
            {
                responder.respond(respond(400, "text/plain", b"bad photo path".to_vec()));
                return;
            }
            let chapter_id = chapter_id.to_string();
            let filename = filename.to_string();
            let config = app
                .try_state::<AppState>()
                .map(|state| {
                    tauri::async_runtime::block_on(async { state.image_cache.read().await.clone() })
                })
                .unwrap_or(ImageCacheConfig {
                    enabled: false,
                    limit_bytes: 0,
                });
            let cache_root = app
                .path()
                .app_cache_dir()
                .unwrap_or_default()
                .join("imgcache");
            let key = format!("photo/{chapter_id}/{filename}");
            let hex = hex_prefix(&key);
            let cache_file = cache_root.join(&hex[..2]).join(&hex);
            if config.enabled {
                if let Ok(bytes) = std::fs::read(&cache_file) {
                    responder.respond(respond(200, guess_image_content_type(&bytes), bytes));
                    return;
                }
            }
            tauri::async_runtime::spawn(async move {
                let Ok(_permit) = image_download_limit().acquire().await else {
                    responder.respond(respond(503, "text/plain", b"busy".to_vec()));
                    return;
                };
                let Some(state) = app.try_state::<AppState>() else {
                    responder.respond(respond(500, "text/plain", b"state unavailable".to_vec()));
                    return;
                };
                let client = state.client.clone();
                let used_counter = state.image_cache_used.clone();
                match client.display_image(&chapter_id, &filename).await {
                    Ok((bytes, content_type)) => {
                        if config.enabled && bytes.len() <= 8 * 1024 * 1024 {
                            let dir_ready = cache_file
                                .parent()
                                .map(|parent| std::fs::create_dir_all(parent).is_ok())
                                .unwrap_or(false);
                            let written = dir_ready && std::fs::write(&cache_file, &bytes).is_ok();
                            if written {
                                let size = bytes.len() as u64;
                                let used = used_counter
                                    .fetch_add(size, std::sync::atomic::Ordering::Relaxed)
                                    + size;
                                if config.limit_bytes > 0 && used > config.limit_bytes {
                                    evict_cache(
                                        &cache_root,
                                        &used_counter,
                                        config.limit_bytes / 10 * 9,
                                    );
                                }
                            }
                        }
                        responder.respond(respond(200, content_type, bytes));
                    }
                    Err(error) => {
                        log::warn!("jmimg photo failed for {chapter_id}/{filename}: {error}");
                        responder.respond(respond(502, "text/plain", b"fetch failed".to_vec()));
                    }
                }
            });
        }
        _ => responder.respond(respond(400, "text/plain", b"unknown kind".to_vec())),
    }
}

fn hex_prefix(value: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn image_download_limit() -> &'static tokio::sync::Semaphore {
    static LIMIT: std::sync::OnceLock<tokio::sync::Semaphore> = std::sync::OnceLock::new();
    LIMIT.get_or_init(|| tokio::sync::Semaphore::new(8))
}

pub(crate) fn dir_used_bytes(dir: &std::path::Path) -> u64 {
    fn walk(dir: &std::path::Path, total: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                walk(&entry.path(), total);
            } else {
                *total += meta.len();
            }
        }
    }
    let mut total = 0;
    walk(dir, &mut total);
    total
}

fn evict_cache(root: &std::path::Path, used: &std::sync::atomic::AtomicU64, target: u64) {
    use std::sync::atomic::Ordering;
    fn collect(
        dir: &std::path::Path,
        files: &mut Vec<(std::time::SystemTime, u64, std::path::PathBuf)>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if meta.is_dir() {
                collect(&entry.path(), files);
            } else {
                files.push((
                    meta.modified().unwrap_or(std::time::UNIX_EPOCH),
                    meta.len(),
                    entry.path(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    collect(root, &mut files);
    files.sort_by_key(|(mtime, _, _)| *mtime);
    let mut current = used.load(Ordering::Relaxed);
    for (_, len, path) in files {
        if current <= target {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            used.fetch_sub(len.min(current), Ordering::Relaxed);
            current = current.saturating_sub(len);
        }
    }
    log::info!("imgcache evicted down to {current} bytes (target {target})");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("jmimg", |ctx, request, responder| {
            handle_jmimg(ctx.app_handle().clone(), request, responder)
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&app_dir).ok();

            let db_path = app_dir.join("jm.db").to_string_lossy().to_string();
            let default_download_dir = app_dir.join("downloads").to_string_lossy().to_string();

            let (db, download_dir, proxy, avs, login_domain, user_id, api_domain, img_host) =
                tauri::async_runtime::block_on(async {
                    let db = db::Database::new(&db_path).await?;
                    let _ = db.mark_interrupted_downloads().await;
                    let download_dir = db
                        .get_setting("download_dir")
                        .await
                        .ok()
                        .flatten()
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or(default_download_dir);
                    let proxy = db.get_setting("proxy_url").await.ok().flatten();
                    let avs = db.get_setting("jm_avs").await.ok().flatten();
                    let login_domain = db.get_setting("jm_login_domain").await.ok().flatten();
                    let user_id = db.get_setting("jm_user_id").await.ok().flatten();
                    let api_domain = db.get_setting("api_domain").await.ok().flatten();
                    let img_host = db.get_setting("img_host").await.ok().flatten();
                    Ok::<_, sqlx::Error>((
                        Arc::new(db),
                        download_dir,
                        proxy,
                        avs,
                        login_domain,
                        user_id,
                        api_domain,
                        img_host,
                    ))
                })?;

            let client = Arc::new(api::JmClient::new());
            if let Some(proxy) = proxy.filter(|value| !value.trim().is_empty()) {
                tauri::async_runtime::block_on(client.set_proxy(Some(proxy))).ok();
            }
            if let Some(domain) = api_domain.filter(|value| !value.trim().is_empty()) {
                tauri::async_runtime::block_on(client.set_api_domain(&domain)).ok();
            }
            tauri::async_runtime::block_on(client.set_session(
                avs.filter(|value| !value.trim().is_empty()),
                login_domain.filter(|value| !value.trim().is_empty()),
                user_id.filter(|value| !value.trim().is_empty()),
            ));
            if let Some(host) = img_host.filter(|value| !value.trim().is_empty()) {
                tauri::async_runtime::block_on(client.set_img_host(host));
            }
            {
                let client = client.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = client.bootstrap().await {
                        log::warn!("JM /setting bootstrap failed: {error}");
                    }
                });
            }

            let image_cache_enabled = tauri::async_runtime::block_on(
                db.get_setting("image_cache_enabled"),
            )
            .ok()
            .flatten()
            .map(|value| value == "1")
            .unwrap_or(false);
            let image_cache_limit_mb: u64 =
                tauri::async_runtime::block_on(db.get_setting("image_cache_limit_mb"))
                    .ok()
                    .flatten()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(256)
                    .clamp(16, 8192);
            let image_cache_used = app
                .path()
                .app_cache_dir()
                .map(|path| dir_used_bytes(&path.join("imgcache")))
                .unwrap_or(0);

            let download_manager = Arc::new(download::DownloadManager::new(3));

            {
                let db = db.clone();
                tauri::async_runtime::spawn(async move {
                    match db.backfill_download_history().await {
                        Ok(count) if count > 0 => {
                            log::info!("Backfilled {count} episode records into download history")
                        }
                        Ok(_) => {}
                        Err(error) => log::warn!("Download history backfill failed: {error}"),
                    }
                    match db.cleanup_missing_downloads().await {
                        Ok(count) if count > 0 => {
                            log::info!("Cleaned {count} episodes whose local files are gone")
                        }
                        Ok(_) => {}
                        Err(error) => log::warn!("Library cleanup failed: {error}"),
                    }
                });
            }

            app.manage(AppState {
                client,
                db,
                download_manager,
                download_dir: Arc::new(RwLock::new(download_dir)),
                image_cache: Arc::new(RwLock::new(ImageCacheConfig {
                    enabled: image_cache_enabled,
                    limit_bytes: image_cache_limit_mb * 1024 * 1024,
                })),
                image_cache_used: Arc::new(std::sync::atomic::AtomicU64::new(image_cache_used)),
            });

            log::info!("JM Client initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            logout,
            is_logged_in,
            get_my_profile,
            check_for_updates,
            open_update_download,
            search_comics,
            get_categories,
            get_category_comics,
            get_ranking,
            get_latest_comics,
            get_weekly_info,
            get_weekly,
            get_promote,
            get_promote_list,
            get_comic_info,
            get_comic_bundle,
            get_episodes,
            get_pictures_page,
            get_local_pictures,
            get_forum,
            post_comment,
            toggle_like,
            toggle_favourite,
            get_my_favourites,
            move_favourite,
            get_watch_list,
            get_daily,
            daily_checkin,
            start_download,
            pause_download,
            resume_download,
            cancel_download,
            get_download_queue,
            get_download_history,
            get_downloaded_chapter_ids,
            get_downloaded_episode_orders,
            get_history_chapter_ids,
            get_history_episode_orders,
            get_download_state_sets,
            clear_finished_download_tasks,
            cleanup_library,
            get_library,
            get_library_stats,
            get_download_dir,
            set_download_dir,
            pick_download_dir,
            get_proxy,
            set_proxy,
            get_api_domain,
            get_api_domains,
            get_api_version,
            set_api_domain,
            refresh_api_domains,
            get_img_host,
            set_img_host,
            get_download_settings,
            save_download_settings,
            get_setting,
            save_setting,
            export_download_history,
            import_download_history,
            get_image_cache_status,
            set_image_cache_settings,
            clear_image_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
