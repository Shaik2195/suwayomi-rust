use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{error, info};
use reqwest::Client;
use sqlx::SqlitePool;

use suwayomi_core::models::DownloadStatus;
use suwayomi_db::repositories::{MangaRepository, ChapterRepository};
use suwayomi_core::traits::MangaSource;

use crate::queue::DownloadQueue;
use crate::storage::ChapterStorage;

pub struct DownloadWorkerPool {
    queue: DownloadQueue,
    storage: ChapterStorage,
    pool: Arc<SqlitePool>,
    source: Arc<dyn MangaSource>,
    client: Client,
    worker_handles: Mutex<Vec<JoinHandle<()>>>,
}

impl DownloadWorkerPool {
    pub fn new(
        queue: DownloadQueue,
        storage: ChapterStorage,
        pool: Arc<SqlitePool>,
        source: Arc<dyn MangaSource>,
    ) -> Self {
        Self {
            queue,
            storage,
            pool,
            source,
            client: Client::new(),
            worker_handles: Mutex::new(Vec::new()),
        }
    }

    pub async fn start(&self, num_workers: usize) {
        let mut handles = self.worker_handles.lock().await;
        for i in 0..num_workers {
            let queue = self.queue.clone();
            let storage = self.storage.clone();
            let pool = self.pool.clone();
            let source = self.source.clone();
            let client = self.client.clone();

            let handle = tokio::spawn(async move {
                Self::worker_loop(i, queue, storage, pool, source, client).await;
            });
            handles.push(handle);
        }
    }

    async fn worker_loop(
        worker_id: usize,
        queue: DownloadQueue,
        storage: ChapterStorage,
        pool: Arc<SqlitePool>,
        source: Arc<dyn MangaSource>,
        client: Client,
    ) {
        use suwayomi_core::models::DownloadEvent;
        
        info!("Worker {} started", worker_id);
        loop {
            if let Some(item) = queue.dequeue().await {
                info!("Worker {} processing chapter {}", worker_id, item.chapter_id);
                
                let result = Self::process_download(
                    &queue,
                    &storage,
                    &pool,
                    &source,
                    &client,
                    item.manga_id,
                    item.chapter_id,
                ).await;

                match result {
                    Ok(_) => {
                        queue.update_status(item.chapter_id, DownloadStatus::Downloaded).await;
                        queue.emit_event(DownloadEvent::Completed(item.chapter_id)).await;
                        info!("Worker {} finished chapter {}", worker_id, item.chapter_id);
                    }
                    Err(e) => {
                        error!("Worker {} failed chapter {}: {}", worker_id, item.chapter_id, e);
                        queue.update_status(item.chapter_id, DownloadStatus::Error).await;
                        queue.emit_event(DownloadEvent::Failed {
                            item_id: item.chapter_id,
                            error: e.to_string(),
                        }).await;
                    }
                }
            } else {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    async fn process_download(
        queue: &DownloadQueue,
        storage: &ChapterStorage,
        pool: &SqlitePool,
        source: &Arc<dyn MangaSource>,
        client: &Client,
        manga_id: i64,
        chapter_id: i64,
    ) -> suwayomi_core::error::Result<()> {
        use suwayomi_core::models::DownloadEvent;

        let manga_repo = MangaRepository::new(pool);
        let chapter_repo = ChapterRepository::new(pool);

        let manga = manga_repo.get_by_id(manga_id).await?
            .ok_or_else(|| suwayomi_core::error::SuwayomiError::NotFound(format!("Manga {}", manga_id)))?;
            
        let chapter = chapter_repo.get_by_id(chapter_id).await?
            .ok_or_else(|| suwayomi_core::error::SuwayomiError::NotFound(format!("Chapter {}", chapter_id)))?;

        let pages = source.get_page_list(&chapter).await?;
        let total_pages = pages.len() as i32;
        
        for (idx, page) in pages.into_iter().enumerate() {
            queue.emit_event(DownloadEvent::Progress {
                item_id: chapter_id,
                page: idx as i32,
                total_pages,
            }).await;
            
            let image_url = page.image_url.clone().unwrap_or(page.url.clone());
            
            let response = client.get(&image_url).send().await
                .map_err(|e| suwayomi_core::error::SuwayomiError::Network(e.to_string()))?;
                
            let bytes = response.bytes().await
                .map_err(|e| suwayomi_core::error::SuwayomiError::Network(e.to_string()))?;

            storage.save_page(&manga.title, &chapter.name, page.index, &bytes).await?;
        }

        Ok(())
    }
}
