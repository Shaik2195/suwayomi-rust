use std::path::{Path, PathBuf};
use suwayomi_core::error::{Result, SuwayomiError};
use tokio::fs;

#[derive(Clone, Debug)]
pub struct ChapterStorage {
    base_dir: PathBuf,
}

impl ChapterStorage {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn sanitize_filename(name: &str) -> String {
        name.chars()
            .map(|c| match c {
                '/' | '\\' | '?' | '%' | '*' | ':' | '|' | '"' | '<' | '>' => '_',
                _ => c,
            })
            .collect()
    }

    pub async fn get_chapter_dir(&self, manga_title: &str, chapter_name: &str) -> PathBuf {
        let safe_manga = Self::sanitize_filename(manga_title);
        let safe_chapter = Self::sanitize_filename(chapter_name);
        self.base_dir.join(safe_manga).join(safe_chapter)
    }

    pub async fn save_page(
        &self,
        manga_title: &str,
        chapter_name: &str,
        page_index: i32,
        bytes: &[u8],
    ) -> Result<PathBuf> {
        let chapter_dir = self.get_chapter_dir(manga_title, chapter_name).await;

        fs::create_dir_all(&chapter_dir).await.map_err(|e| {
            SuwayomiError::Internal(format!("Failed to create chapter directory: {}", e))
        })?;

        let ext = match image::guess_format(bytes) {
            Ok(image::ImageFormat::Png) => "png",
            Ok(image::ImageFormat::WebP) => "webp",
            Ok(image::ImageFormat::Gif) => "gif",
            Ok(image::ImageFormat::Jpeg) => "jpg",
            _ => "jpg",
        };

        let file_name = format!("{:03}.{}", page_index, ext);
        let file_path = chapter_dir.join(file_name);

        fs::write(&file_path, bytes)
            .await
            .map_err(|e| SuwayomiError::Internal(format!("Failed to write page file: {}", e)))?;

        Ok(file_path)
    }

    pub async fn is_chapter_downloaded(&self, manga_title: &str, chapter_name: &str) -> bool {
        let chapter_dir = self.get_chapter_dir(manga_title, chapter_name).await;
        chapter_dir.exists() && chapter_dir.is_dir()
    }

    pub async fn delete_chapter(&self, manga_title: &str, chapter_name: &str) -> Result<()> {
        let chapter_dir = self.get_chapter_dir(manga_title, chapter_name).await;
        if chapter_dir.exists() {
            fs::remove_dir_all(&chapter_dir).await.map_err(|e| {
                SuwayomiError::Internal(format!("Failed to delete chapter directory: {}", e))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            ChapterStorage::sanitize_filename("My Manga/Test: <Title>"),
            "My Manga_Test_ _Title_"
        );
        assert_eq!(
            ChapterStorage::sanitize_filename("Normal Title"),
            "Normal Title"
        );
    }

    #[tokio::test]
    async fn test_storage_operations() {
        let dir = tempdir().unwrap();
        let storage = ChapterStorage::new(dir.path());

        let manga = "Test Manga";
        let chapter = "Chapter 1";

        assert!(!storage.is_chapter_downloaded(manga, chapter).await);

        let data = b"dummy image data";
        let path = storage.save_page(manga, chapter, 0, data).await.unwrap();

        assert!(path.exists());
        assert!(storage.is_chapter_downloaded(manga, chapter).await);

        storage.delete_chapter(manga, chapter).await.unwrap();
        assert!(!storage.is_chapter_downloaded(manga, chapter).await);
    }
}
