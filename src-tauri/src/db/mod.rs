use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;

use crate::api::{Comic, DownloadHistoryEntry, DownloadTask, LibraryStats, LocalComic};

fn earliest(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y < x { y } else { x }),
        (x, y) => x.or(y),
    }
}

fn latest(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(x), Some(y)) => Some(if y > x { y } else { x }),
        (x, y) => x.or(y),
    }
}

fn parse_id_list(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn dir_size(path: &Path) -> i64 {
    fn walk(dir: &Path, total: &mut i64) {
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
                *total += meta.len() as i64;
            }
        }
    }
    let mut total = 0;
    walk(path, &mut total);
    total
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(db_path: &str) -> Result<Self, sqlx::Error> {
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        let db = Self { pool };
        db.init_tables().await?;
        Ok(db)
    }

    async fn init_tables(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS comics (
                comic_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                author TEXT,
                finished INTEGER DEFAULT 0,
                pages_count INTEGER DEFAULT 0,
                category TEXT,
                eps_count INTEGER DEFAULT 0,
                update_time TEXT,
                cover_path TEXT,
                downloaded_at TEXT DEFAULT (datetime('now')),
                created_at TEXT DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS episodes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                comic_id TEXT NOT NULL,
                chapter_id TEXT NOT NULL,
                episode_order INTEGER NOT NULL,
                title TEXT NOT NULL,
                downloaded INTEGER DEFAULT 0,
                image_count INTEGER DEFAULT 0,
                downloaded_images INTEGER DEFAULT 0,
                save_path TEXT,
                downloaded_at TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                UNIQUE(comic_id, chapter_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS download_queue (
                id TEXT PRIMARY KEY,
                comic_id TEXT NOT NULL,
                comic_title TEXT NOT NULL,
                chapter_id TEXT NOT NULL,
                episode_order INTEGER NOT NULL,
                episode_title TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                total_images INTEGER DEFAULT 0,
                downloaded_images INTEGER DEFAULT 0,
                save_path TEXT,
                error TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS download_history (
                comic_id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                chapter_ids TEXT NOT NULL DEFAULT '[]',
                first_downloaded_at TEXT,
                last_downloaded_at TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_comics_title ON comics(title)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_comics_category ON comics(category)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_episodes_comic_id ON episodes(comic_id)")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_episodes_comic_chapter ON episodes(comic_id, chapter_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_download_queue_status ON download_queue(status)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_comic(&self, comic: &Comic) -> Result<(), sqlx::Error> {
        let category = comic.tags.join(",");
        let cover_path = comic.thumb.as_ref().map(|thumb| thumb.path.clone());
        sqlx::query(
            r#"
            INSERT INTO comics (comic_id, title, author, finished, pages_count, category, eps_count, update_time, cover_path)
            VALUES (?, ?, ?, 0, ?, ?, ?, ?, ?)
            ON CONFLICT(comic_id) DO UPDATE SET
                title = excluded.title,
                author = excluded.author,
                pages_count = excluded.pages_count,
                category = excluded.category,
                eps_count = excluded.eps_count,
                update_time = excluded.update_time,
                cover_path = excluded.cover_path
            "#,
        )
        .bind(&comic.id)
        .bind(&comic.title)
        .bind(&comic.author)
        .bind(comic.pages_count)
        .bind(&category)
        .bind(comic.eps_count)
        .bind(&comic.updated_at)
        .bind(&cover_path)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_comics(&self, limit: i64, offset: i64) -> Result<Vec<LocalComic>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM comics ORDER BY downloaded_at DESC LIMIT ? OFFSET ?")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        let mut comics = Vec::new();
        for row in rows {
            let comic_id: String = row.get("comic_id");
            let episodes = self.get_downloaded_episodes(&comic_id).await?;
            comics.push(LocalComic {
                comic_id,
                title: row.get("title"),
                author: row.get("author"),
                pages_count: row.get::<Option<i64>, _>("pages_count"),
                category: row.get("category"),
                eps_count: row.get::<Option<i64>, _>("eps_count"),
                update_time: row.get("update_time"),
                downloaded_episodes: Some(episodes),
                cover_path: row.get("cover_path"),
            });
        }
        Ok(comics)
    }

    pub async fn save_episode(
        &self,
        comic_id: &str,
        chapter_id: &str,
        episode_order: i64,
        title: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO episodes (comic_id, chapter_id, episode_order, title)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(comic_id, chapter_id) DO UPDATE SET
                episode_order = excluded.episode_order,
                title = excluded.title
            "#,
        )
        .bind(comic_id)
        .bind(chapter_id)
        .bind(episode_order)
        .bind(title)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_episode_save_path(
        &self,
        comic_id: &str,
        chapter_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT save_path FROM episodes WHERE comic_id = ? AND chapter_id = ? AND downloaded = 1",
        )
        .bind(comic_id)
        .bind(chapter_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|row| row.get::<Option<String>, _>("save_path")))
    }

    pub async fn mark_episode_downloaded(
        &self,
        comic_id: &str,
        chapter_id: &str,
        image_count: i64,
        save_path: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE episodes
            SET downloaded = 1,
                image_count = ?,
                downloaded_images = ?,
                save_path = ?,
                downloaded_at = datetime('now')
            WHERE comic_id = ? AND chapter_id = ?
            "#,
        )
        .bind(image_count)
        .bind(image_count)
        .bind(save_path)
        .bind(comic_id)
        .bind(chapter_id)
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE comics SET downloaded_at = datetime('now') WHERE comic_id = ?")
            .bind(comic_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_downloaded_episodes(&self, comic_id: &str) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT title FROM episodes WHERE comic_id = ? AND downloaded = 1 ORDER BY episode_order",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| row.get("title")).collect())
    }

    pub async fn get_downloaded_chapter_ids(
        &self,
        comic_id: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT chapter_id FROM episodes WHERE comic_id = ? AND downloaded = 1 ORDER BY episode_order",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| row.get("chapter_id")).collect())
    }

    pub async fn get_downloaded_episode_orders(
        &self,
        comic_id: &str,
    ) -> Result<Vec<i64>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT episode_order FROM episodes WHERE comic_id = ? AND downloaded = 1 ORDER BY episode_order",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|row| row.get("episode_order")).collect())
    }

    pub async fn upsert_download_history(
        &self,
        comic_id: &str,
        title: &str,
        chapter_id: &str,
    ) -> Result<(), sqlx::Error> {
        let row = sqlx::query("SELECT chapter_ids FROM download_history WHERE comic_id = ?")
            .bind(comic_id)
            .fetch_optional(&self.pool)
            .await?;
        let mut ids = row
            .map(|row| parse_id_list(&row.get::<String, _>("chapter_ids")))
            .unwrap_or_default();
        if !ids.iter().any(|id| id == chapter_id) {
            ids.push(chapter_id.to_string());
        }
        let encoded = serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            r#"
            INSERT INTO download_history (comic_id, title, chapter_ids, first_downloaded_at, last_downloaded_at)
            VALUES (?, ?, ?, datetime('now'), datetime('now'))
            ON CONFLICT(comic_id) DO UPDATE SET
                title = excluded.title,
                chapter_ids = excluded.chapter_ids,
                last_downloaded_at = datetime('now')
            "#,
        )
        .bind(comic_id)
        .bind(title)
        .bind(encoded)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_download_history(&self) -> Result<Vec<DownloadHistoryEntry>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM download_history ORDER BY last_downloaded_at DESC")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| DownloadHistoryEntry {
                comic_id: row.get("comic_id"),
                title: row.get("title"),
                chapter_ids: parse_id_list(&row.get::<String, _>("chapter_ids")),
                first_downloaded_at: row.get("first_downloaded_at"),
                last_downloaded_at: row.get("last_downloaded_at"),
            })
            .collect())
    }

    pub async fn get_history_chapter_ids(
        &self,
        comic_id: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        let row = sqlx::query("SELECT chapter_ids FROM download_history WHERE comic_id = ?")
            .bind(comic_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row
            .map(|row| parse_id_list(&row.get::<String, _>("chapter_ids")))
            .unwrap_or_default())
    }

    pub async fn get_history_episode_orders(
        &self,
        comic_id: &str,
    ) -> Result<Vec<i64>, sqlx::Error> {
        let chapter_ids = self.get_history_chapter_ids(comic_id).await?;
        if chapter_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query("SELECT chapter_id, episode_order FROM episodes WHERE comic_id = ?")
            .bind(comic_id)
            .fetch_all(&self.pool)
            .await?;
        let map: HashMap<String, i64> = rows
            .into_iter()
            .map(|row| (row.get("chapter_id"), row.get("episode_order")))
            .collect();
        Ok(chapter_ids
            .into_iter()
            .filter_map(|id| map.get(&id).copied())
            .collect())
    }

    pub async fn backfill_download_history(&self) -> Result<usize, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT comic_id, chapter_id FROM episodes WHERE downloaded = 1 ORDER BY comic_id, episode_order",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
        for row in rows {
            grouped
                .entry(row.get("comic_id"))
                .or_default()
                .push(row.get("chapter_id"));
        }
        let mut count = 0;
        for (comic_id, chapter_ids) in grouped {
            let title_row = sqlx::query("SELECT title FROM comics WHERE comic_id = ?")
                .bind(&comic_id)
                .fetch_optional(&self.pool)
                .await?;
            let title = title_row
                .map(|row| row.get::<String, _>("title"))
                .unwrap_or_else(|| comic_id.clone());
            let existing = self.get_history_chapter_ids(&comic_id).await?;
            let mut merged = existing;
            for id in chapter_ids {
                if !merged.contains(&id) {
                    merged.push(id);
                }
            }
            let encoded = serde_json::to_string(&merged).unwrap_or_else(|_| "[]".to_string());
            sqlx::query(
                r#"
                INSERT INTO download_history (comic_id, title, chapter_ids, first_downloaded_at, last_downloaded_at)
                VALUES (?, ?, ?, datetime('now'), datetime('now'))
                ON CONFLICT(comic_id) DO UPDATE SET
                    title = excluded.title,
                    chapter_ids = excluded.chapter_ids,
                    last_downloaded_at = datetime('now')
                "#,
            )
            .bind(&comic_id)
            .bind(&title)
            .bind(encoded)
            .execute(&self.pool)
            .await?;
            count += 1;
        }
        Ok(count)
    }

    pub async fn merge_download_history(
        &self,
        entries: &[DownloadHistoryEntry],
    ) -> Result<usize, sqlx::Error> {
        for entry in entries {
            let existing = sqlx::query("SELECT * FROM download_history WHERE comic_id = ?")
                .bind(&entry.comic_id)
                .fetch_optional(&self.pool)
                .await?;
            let (mut ids, first, last) = existing
                .map(|row| {
                    (
                        parse_id_list(&row.get::<String, _>("chapter_ids")),
                        row.get::<Option<String>, _>("first_downloaded_at"),
                        row.get::<Option<String>, _>("last_downloaded_at"),
                    )
                })
                .unwrap_or_default();
            for id in &entry.chapter_ids {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
            let encoded = serde_json::to_string(&ids).unwrap_or_else(|_| "[]".to_string());
            let first = earliest(first, entry.first_downloaded_at.clone());
            let last = latest(last, entry.last_downloaded_at.clone());
            sqlx::query(
                r#"
                INSERT INTO download_history (comic_id, title, chapter_ids, first_downloaded_at, last_downloaded_at)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(comic_id) DO UPDATE SET
                    title = excluded.title,
                    chapter_ids = excluded.chapter_ids,
                    first_downloaded_at = excluded.first_downloaded_at,
                    last_downloaded_at = excluded.last_downloaded_at
                "#,
            )
            .bind(&entry.comic_id)
            .bind(&entry.title)
            .bind(encoded)
            .bind(first)
            .bind(last)
            .execute(&self.pool)
            .await?;
        }
        Ok(entries.len())
    }

    pub async fn add_download_task(&self, task: &DownloadTask) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO download_queue (id, comic_id, comic_title, chapter_id, episode_order, episode_title, status, total_images, downloaded_images, save_path)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                total_images = excluded.total_images,
                downloaded_images = excluded.downloaded_images,
                save_path = excluded.save_path,
                updated_at = datetime('now')
            "#,
        )
        .bind(&task.id)
        .bind(&task.comic_id)
        .bind(&task.comic_title)
        .bind(&task.chapter_id)
        .bind(task.episode_order)
        .bind(&task.episode_title)
        .bind(&task.status)
        .bind(task.total_images)
        .bind(task.downloaded_images)
        .bind(&task.save_path)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_download_task(
        &self,
        task_id: &str,
        downloaded: i64,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE download_queue SET downloaded_images = ?, status = ?, error = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(downloaded)
        .bind(status)
        .bind(error)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_download_status(
        &self,
        task_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE download_queue SET status = ?, error = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(status)
        .bind(error)
        .bind(task_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_download_total(&self, task_id: &str, total: i64) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE download_queue SET total_images = ? WHERE id = ?")
            .bind(total)
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_interrupted_downloads(&self) -> Result<usize, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE download_queue SET status = 'failed', error = '应用重启，任务已中断', updated_at = datetime('now') WHERE status IN ('pending','downloading','paused')",
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn has_active_downloads(&self) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM download_queue WHERE status IN ('pending', 'downloading'))",
        )
        .fetch_one(&self.pool)
        .await
    }

    pub async fn get_all_downloads(&self, limit: i64) -> Result<Vec<DownloadTask>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM download_queue ORDER BY created_at DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(row_to_task).collect())
    }

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| row.get("value")))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO settings (key, value) VALUES (?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 清空全部本地数据（设置、下载记录、本地书库缓存），用于“清除所有数据”。
    pub async fn wipe_all(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for table in [
            "download_queue",
            "episodes",
            "comics",
            "download_history",
            "settings",
        ] {
            sqlx::query(&format!("DELETE FROM {table}"))
                .execute(&mut *tx)
                .await?;
        }
        // 重置自增序列，让清除后的状态与全新安装一致。
        sqlx::query("DELETE FROM sqlite_sequence")
            .execute(&mut *tx)
            .await
            .ok();
        tx.commit().await?;
        // 回收磁盘空间；失败不影响清除结果。
        let _ = sqlx::query("VACUUM").execute(&self.pool).await;
        Ok(())
    }

    pub async fn cleanup_missing_downloads(&self) -> Result<usize, sqlx::Error> {
        let rows = sqlx::query("SELECT comic_id, chapter_id, save_path FROM episodes WHERE downloaded = 1")
            .fetch_all(&self.pool)
            .await?;
        let mut cleaned = 0;
        for row in rows {
            let save_path: Option<String> = row.get("save_path");
            let exists = save_path
                .as_ref()
                .map(|path| Path::new(path).is_dir())
                .unwrap_or(false);
            if !exists {
                sqlx::query(
                    "UPDATE episodes SET downloaded = 0, image_count = 0, downloaded_images = 0, save_path = NULL, downloaded_at = NULL WHERE comic_id = ? AND chapter_id = ?",
                )
                .bind(row.get::<String, _>("comic_id"))
                .bind(row.get::<String, _>("chapter_id"))
                .execute(&self.pool)
                .await?;
                cleaned += 1;
            }
        }
        sqlx::query(
            "DELETE FROM comics WHERE comic_id NOT IN (SELECT DISTINCT comic_id FROM episodes WHERE downloaded = 1)",
        )
        .execute(&self.pool)
        .await?;
        Ok(cleaned)
    }

    pub async fn cleanup_missing_downloads_for_comic(
        &self,
        comic_id: &str,
    ) -> Result<usize, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT chapter_id, save_path FROM episodes WHERE comic_id = ? AND downloaded = 1",
        )
        .bind(comic_id)
        .fetch_all(&self.pool)
        .await?;
        let mut cleaned = 0;
        for row in rows {
            let save_path: Option<String> = row.get("save_path");
            let exists = save_path
                .as_ref()
                .map(|path| Path::new(path).is_dir())
                .unwrap_or(false);
            if !exists {
                sqlx::query(
                    "UPDATE episodes SET downloaded = 0, image_count = 0, downloaded_images = 0, save_path = NULL, downloaded_at = NULL WHERE comic_id = ? AND chapter_id = ?",
                )
                .bind(comic_id)
                .bind(row.get::<String, _>("chapter_id"))
                .execute(&self.pool)
                .await?;
                cleaned += 1;
            }
        }
        Ok(cleaned)
    }

    pub async fn clear_finished_download_tasks(&self) -> Result<usize, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM download_queue WHERE status IN ('completed','failed','cancelled')",
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    pub async fn get_download_state_sets(
        &self,
    ) -> Result<(Vec<String>, Vec<String>), sqlx::Error> {
        let history = sqlx::query("SELECT comic_id FROM download_history")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|row| row.get("comic_id"))
            .collect();
        let local = sqlx::query(
            "SELECT DISTINCT comic_id FROM episodes WHERE downloaded = 1",
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| row.get("comic_id"))
        .collect();
        Ok((history, local))
    }

    pub async fn get_stats(&self) -> Result<LibraryStats, sqlx::Error> {
        let comics = sqlx::query("SELECT COUNT(*) as count FROM comics")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count");
        let episodes = sqlx::query("SELECT COUNT(*) as count FROM episodes WHERE downloaded = 1")
            .fetch_one(&self.pool)
            .await?
            .get::<i64, _>("count");
        let images = sqlx::query(
            "SELECT COALESCE(SUM(image_count), 0) as count FROM episodes WHERE downloaded = 1",
        )
        .fetch_one(&self.pool)
        .await?
        .get::<i64, _>("count");
        let rows = sqlx::query(
            "SELECT DISTINCT save_path FROM episodes WHERE downloaded = 1 AND save_path IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut seen = HashSet::new();
        let mut total_size_bytes = 0;
        for row in rows {
            let path: String = row.get("save_path");
            if seen.insert(path.clone()) {
                total_size_bytes += dir_size(Path::new(&path));
            }
        }
        Ok(LibraryStats {
            total_comics: comics,
            total_episodes: episodes,
            total_images: images,
            total_size_bytes,
        })
    }
}

fn row_to_task(row: sqlx::sqlite::SqliteRow) -> DownloadTask {
    DownloadTask {
        id: row.get("id"),
        comic_id: row.get("comic_id"),
        comic_title: row.get("comic_title"),
        chapter_id: row.get("chapter_id"),
        episode_order: row.get("episode_order"),
        episode_title: row.get("episode_title"),
        status: row.get("status"),
        total_images: row.get("total_images"),
        downloaded_images: row.get("downloaded_images"),
        save_path: row.get::<Option<String>, _>("save_path").unwrap_or_default(),
        created_at: row.get("created_at"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn updater_blocks_only_running_or_queued_comic_downloads() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db = Database { pool };
        db.init_tables().await.unwrap();
        assert!(!db.has_active_downloads().await.unwrap());
        sqlx::query("INSERT INTO download_queue (id, comic_id, comic_title, chapter_id, episode_order, episode_title, status) VALUES ('test', 'comic', '漫画', 'chapter', 1, '章节', 'pending')")
            .execute(&db.pool).await.unwrap();
        for status in ["pending", "downloading", "paused", "completed", "failed", "cancelled"] {
            db.set_download_status("test", status, None).await.unwrap();
            assert_eq!(db.has_active_downloads().await.unwrap(), matches!(status, "pending" | "downloading"), "{status}");
        }
        db.pool.close().await;
    }

    #[tokio::test]
    async fn wipe_all_clears_settings_and_download_records() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let db = Database { pool };
        db.init_tables().await.unwrap();
        db.set_setting("jm_login_domain", "example.net")
            .await
            .unwrap();
        sqlx::query("INSERT INTO comics (comic_id, title) VALUES ('c1', '漫画')")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO download_queue (id, comic_id, comic_title, chapter_id, episode_order, episode_title, status) VALUES ('t1', 'c1', '漫画', 'ch1', 1, '章节', 'completed')")
            .execute(&db.pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO download_history (comic_id, title) VALUES ('c1', '漫画')")
            .execute(&db.pool)
            .await
            .unwrap();

        db.wipe_all().await.unwrap();

        assert_eq!(db.get_setting("jm_login_domain").await.unwrap(), None);
        assert!(db.get_comics(10, 0).await.unwrap().is_empty());
        assert!(db.get_all_downloads(10).await.unwrap().is_empty());
        assert!(db.get_download_history().await.unwrap().is_empty());
        db.pool.close().await;
    }
}
