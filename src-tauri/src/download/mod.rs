use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, Notify, Semaphore};

use crate::api::{scramble, JmClient};
use crate::db::Database;

const TASK_RUNNING: u8 = 0;
const TASK_PAUSED: u8 = 1;
const TASK_CANCELLED: u8 = 2;

pub struct TaskControl {
    state: AtomicU8,
    notify: Notify,
}

impl TaskControl {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(TASK_RUNNING),
            notify: Notify::new(),
        }
    }

    fn pause(&self) {
        self.state.store(TASK_PAUSED, Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    fn resume(&self) {
        self.state.store(TASK_RUNNING, Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    fn cancel(&self) {
        self.state.store(TASK_CANCELLED, Ordering::Relaxed);
        self.notify.notify_waiters();
    }

    fn is_cancelled(&self) -> bool {
        self.state.load(Ordering::Relaxed) == TASK_CANCELLED
    }

    /// Returns false when the task has been cancelled.
    async fn wait_until_runnable(&self) -> bool {
        loop {
            match self.state.load(Ordering::Relaxed) {
                TASK_RUNNING => return true,
                TASK_CANCELLED => return false,
                _ => self.notify.notified().await,
            }
        }
    }
}

pub struct DownloadManager {
    semaphore: Arc<Semaphore>,
    controls: Arc<Mutex<HashMap<String, Arc<TaskControl>>>>,
}

impl DownloadManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
            controls: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn register(&self, task_id: &str) {
        self.controls
            .lock()
            .await
            .insert(task_id.to_string(), Arc::new(TaskControl::new()));
    }

    async fn control(&self, task_id: &str) -> Option<Arc<TaskControl>> {
        self.controls.lock().await.get(task_id).cloned()
    }

    pub async fn pause(&self, task_id: &str) -> bool {
        if let Some(control) = self.control(task_id).await {
            control.pause();
            true
        } else {
            false
        }
    }

    pub async fn resume(&self, task_id: &str) -> bool {
        if let Some(control) = self.control(task_id).await {
            control.resume();
            true
        } else {
            false
        }
    }

    pub async fn cancel(&self, task_id: &str) -> bool {
        if let Some(control) = self.control(task_id).await {
            control.cancel();
            true
        } else {
            false
        }
    }

    pub async fn forget(&self, task_id: &str) {
        self.controls.lock().await.remove(task_id);
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn download_episode(
        &self,
        client: Arc<JmClient>,
        db: Arc<Database>,
        task_id: String,
        comic_id: String,
        comic_title: String,
        chapter_id: String,
        chapter_title: String,
        episode_order: i64,
        base_dir: String,
        output_format: String,
        concurrency: usize,
        interval_ms: u64,
    ) {
        if self.control(&task_id).await.is_none() {
            self.register(&task_id).await;
        }
        let Some(control) = self.control(&task_id).await else {
            return;
        };

        let _permit = match self.semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                let _ = db
                    .update_download_task(&task_id, 0, "failed", Some("下载队列已关闭"))
                    .await;
                self.forget(&task_id).await;
                return;
            }
        };

        if !control.wait_until_runnable().await {
            let _ = db
                .update_download_task(&task_id, 0, "cancelled", Some("用户取消"))
                .await;
            self.forget(&task_id).await;
            return;
        }

        let _ = db
            .update_download_task(&task_id, 0, "downloading", None)
            .await;

        // Make sure the comic and all chapters exist locally.
        let album = match client.album_raw(&comic_id).await {
            Ok(album) => album,
            Err(error) => {
                let _ = db
                    .update_download_task(&task_id, 0, "failed", Some(&error.to_string()))
                    .await;
                self.forget(&task_id).await;
                return;
            }
        };
        let img_host = client.img_host().await;
        let comic = crate::api::comic_from_album(album.clone(), &img_host);
        let _ = db.save_comic(&comic).await;
        for episode in crate::api::episodes_from_album(&album) {
            let _ = db
                .save_episode(&comic_id, &episode.id, episode.order, &episode.title)
                .await;
        }

        let scramble_id = client.scramble_id(&chapter_id).await;
        let chapter = match client.chapter(&chapter_id).await {
            Ok(chapter) => chapter,
            Err(error) => {
                let _ = db
                    .update_download_task(&task_id, 0, "failed", Some(&error.to_string()))
                    .await;
                self.forget(&task_id).await;
                return;
            }
        };

        let images: Vec<String> = chapter
            .images
            .into_iter()
            .filter(|filename| {
                filename
                    .rsplit_once('.')
                    .map(|(_, ext)| {
                        matches!(
                            ext.to_ascii_lowercase().as_str(),
                            "webp" | "gif" | "jpg" | "jpeg" | "png"
                        )
                    })
                    .unwrap_or(false)
            })
            .collect();
        let total = images.len() as i64;
        if total == 0 {
            let _ = db
                .update_download_task(&task_id, 0, "failed", Some("本章没有图片"))
                .await;
            self.forget(&task_id).await;
            return;
        }
        let _ = db.set_download_total(&task_id, total).await;

        let comic_dir = Path::new(&base_dir).join(sanitize_filename(&comic_title));
        let safe_chapter = if chapter_title.trim().is_empty() {
            format!("第{episode_order}话")
        } else {
            sanitize_filename(&chapter_title)
        };
        let chapter_dir = comic_dir.join(&safe_chapter);
        let temp_dir = comic_dir.join(format!(".{safe_chapter}.downloading"));
        if let Err(error) = std::fs::create_dir_all(&temp_dir) {
            let _ = db
                .update_download_task(
                    &task_id,
                    0,
                    "failed",
                    Some(&format!("创建目录失败: {error}")),
                )
                .await;
            self.forget(&task_id).await;
            return;
        }

        let downloaded = Arc::new(AtomicI64::new(0));
        let image_sem = Arc::new(Semaphore::new(concurrency.max(1)));
        let mut handles = Vec::new();
        let parsed_chapter = chapter_id.parse::<i64>().unwrap_or(0);
        let format = output_format.clone();

        for (index, filename) in images.iter().cloned().enumerate() {
            let client = client.clone();
            let db = db.clone();
            let control = control.clone();
            let image_sem = image_sem.clone();
            let downloaded = downloaded.clone();
            let temp_dir = temp_dir.clone();
            let task_id = task_id.clone();
            let format = format.clone();
            let chapter_id_for_task = chapter_id.clone();
            let extension_guess = filename
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
                .unwrap_or_else(|| "webp".to_string());
            let target_extension = if extension_guess == "gif" {
                "gif".to_string()
            } else {
                scramble::extension_for(&format).to_string()
            };
            let target_path = temp_dir.join(format!("{:04}.{}", index + 1, target_extension));

            let handle = tokio::spawn(async move {
                let Ok(permit) = image_sem.acquire().await else {
                    return;
                };
                if !control.wait_until_runnable().await {
                    drop(permit);
                    return;
                }
                if target_path.exists() {
                    let count = downloaded.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = db
                        .update_download_task(&task_id, count, "downloading", None)
                        .await;
                    drop(permit);
                    return;
                }

                let mut last_error = None;
                for attempt in 0..3 {
                    if !control.wait_until_runnable().await {
                        drop(permit);
                        return;
                    }
                    let bytes = match client.image_bytes(&chapter_id_for_task, &filename).await {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            last_error = Some(error.to_string());
                            tokio::time::sleep(Duration::from_millis(500 * (attempt + 1))).await;
                            continue;
                        }
                    };
                    let filename_for_processing = filename.clone();
                    let processed = tokio::task::spawn_blocking(move || {
                        scramble::process_for_download(
                            scramble_id,
                            parsed_chapter,
                            &filename_for_processing,
                            &bytes,
                            &format,
                        )
                    })
                    .await;
                    let data = match processed {
                        Ok(Ok((data, _extension))) => data,
                        Ok(Err(error)) => {
                            last_error = Some(error);
                            break;
                        }
                        Err(error) => {
                            last_error = Some(format!("图片处理线程失败: {error}"));
                            break;
                        }
                    };
                    if let Err(error) = std::fs::write(&target_path, data) {
                        last_error = Some(format!("写入失败: {error}"));
                        break;
                    }
                    let count = downloaded.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = db
                        .update_download_task(&task_id, count, "downloading", None)
                        .await;
                    last_error = None;
                    break;
                }
                if let Some(error) = last_error {
                    log::error!("下载失败 {chapter_id_for_task}/{filename}: {error}");
                }
                drop(permit);
                if interval_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(interval_ms)).await;
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        if control.is_cancelled() {
            let count = downloaded.load(Ordering::Relaxed);
            let _ = db
                .update_download_task(&task_id, count, "cancelled", Some("用户取消"))
                .await;
            self.forget(&task_id).await;
            return;
        }

        let count = downloaded.load(Ordering::Relaxed);
        if count == total {
            if chapter_dir.exists() {
                let _ = std::fs::remove_dir_all(&chapter_dir);
            }
            if let Err(error) = std::fs::rename(&temp_dir, &chapter_dir) {
                let _ = db
                    .update_download_task(
                        &task_id,
                        count,
                        "failed",
                        Some(&format!("保存章节目录失败: {error}")),
                    )
                    .await;
                self.forget(&task_id).await;
                return;
            }
            let save_path = chapter_dir.to_string_lossy().to_string();
            let _ = db
                .mark_episode_downloaded(&comic_id, &chapter_id, total, &save_path)
                .await;
            let _ = db
                .upsert_download_history(&comic_id, &comic_title, &chapter_id)
                .await;
            let _ = db
                .update_download_task(&task_id, count, "completed", None)
                .await;
            log::info!("下载完成 {comic_title} - {chapter_title} ({count}/{total})");
        } else {
            let _ = db
                .update_download_task(
                    &task_id,
                    count,
                    "failed",
                    Some(&format!("{count}/{total} 张图片下载成功")),
                )
                .await;
        }
        self.forget(&task_id).await;
    }
}

fn sanitize_filename(name: &str) -> String {
    let replacements = [
        ("/", "／"),
        ("\\", "＼"),
        ("?", "？"),
        ("|", "︱"),
        ("\"", "＂"),
        ("*", "＊"),
        ("<", "＜"),
        (">", "＞"),
        (":", "："),
    ];
    let mut result = name.to_string();
    for (old, new) in replacements {
        result = result.replace(old, new);
    }
    result = result.trim().to_string();
    result.chars().take(180).collect()
}
