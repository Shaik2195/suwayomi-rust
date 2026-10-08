use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use suwayomi_core::models::{DownloadQueueItem, DownloadStatus};

#[derive(Clone)]
pub struct DownloadQueue {
    queue: Arc<RwLock<VecDeque<DownloadQueueItem>>>,
    is_paused: Arc<RwLock<bool>>,
    progress_tx: broadcast::Sender<DownloadQueueItem>,
}

impl DownloadQueue {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            is_paused: Arc::new(RwLock::new(false)),
            progress_tx: tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DownloadQueueItem> {
        self.progress_tx.subscribe()
    }

    pub async fn enqueue(&self, item: DownloadQueueItem) {
        let mut q = self.queue.write().await;
        // avoid duplicates
        if !q.iter().any(|i| i.chapter_id == item.chapter_id) {
            // we maintain queue ordered naturally by priority since it's asked to act like one.
            // But we actually provide `reorder` for exact priority changes. 
            // In a real priority queue, we'd sort by some rank, here we enqueue at back
            // but can move things around.
            q.push_back(item.clone());
            let _ = self.progress_tx.send(item);
        }
    }

    pub async fn dequeue(&self) -> Option<DownloadQueueItem> {
        let is_paused = *self.is_paused.read().await;
        if is_paused {
            return None;
        }

        let mut q = self.queue.write().await;
        if let Some(mut item) = q.pop_front() {
            item.status = DownloadStatus::Downloading;
            let _ = self.progress_tx.send(item.clone());
            Some(item)
        } else {
            None
        }
    }

    pub async fn update_status(&self, chapter_id: i64, status: DownloadStatus) {
        let mut q = self.queue.write().await;
        for item in q.iter_mut() {
            if item.chapter_id == chapter_id {
                item.status = status.clone();
                let _ = self.progress_tx.send(item.clone());
                break;
            }
        }
    }

    pub async fn reorder(&self, chapter_id: i64, new_index: usize) {
        let mut q = self.queue.write().await;
        
        let current_idx_opt = q.iter().position(|i| i.chapter_id == chapter_id);
        
        if let Some(current_idx) = current_idx_opt {
            let item = q.remove(current_idx).unwrap();
            let target_idx = new_index.min(q.len());
            q.insert(target_idx, item);
        }
    }

    pub async fn pause(&self) {
        *self.is_paused.write().await = true;
    }

    pub async fn resume(&self) {
        *self.is_paused.write().await = false;
    }

    pub async fn is_paused(&self) -> bool {
        *self.is_paused.read().await
    }

    pub async fn clear(&self) {
        let mut q = self.queue.write().await;
        q.clear();
    }

    pub async fn get_all(&self) -> Vec<DownloadQueueItem> {
        let q = self.queue.read().await;
        q.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use suwayomi_core::models::DownloadStatus;

    fn create_item(chapter_id: i64) -> DownloadQueueItem {
        DownloadQueueItem {
            chapter_id,
            manga_id: 1,
            status: DownloadStatus::Queued,
        }
    }

    #[tokio::test]
    async fn test_queue_operations() {
        let queue = DownloadQueue::new();
        
        queue.enqueue(create_item(1)).await;
        queue.enqueue(create_item(2)).await;

        let all = queue.get_all().await;
        assert_eq!(all.len(), 2);

        let dequeued = queue.dequeue().await.unwrap();
        assert_eq!(dequeued.chapter_id, 1);
        assert_eq!(dequeued.status, DownloadStatus::Downloading);

        let all_after = queue.get_all().await;
        assert_eq!(all_after.len(), 1);
        
        queue.clear().await;
        assert_eq!(queue.get_all().await.len(), 0);
    }

    #[tokio::test]
    async fn test_reorder() {
        let queue = DownloadQueue::new();
        queue.enqueue(create_item(1)).await;
        queue.enqueue(create_item(2)).await;
        queue.enqueue(create_item(3)).await;

        queue.reorder(3, 0).await;
        let all = queue.get_all().await;
        assert_eq!(all[0].chapter_id, 3);
        assert_eq!(all[1].chapter_id, 1);
        assert_eq!(all[2].chapter_id, 2);
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let queue = DownloadQueue::new();
        queue.enqueue(create_item(1)).await;

        queue.pause().await;
        let item = queue.dequeue().await;
        assert!(item.is_none());

        queue.resume().await;
        let item = queue.dequeue().await;
        assert!(item.is_some());
        assert_eq!(item.unwrap().chapter_id, 1);
    }
}
