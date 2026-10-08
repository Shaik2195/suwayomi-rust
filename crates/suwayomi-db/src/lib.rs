pub mod pool;
pub mod migrations;
pub mod repositories;

#[cfg(test)]
mod tests {
    use super::*;
    use pool::create_sqlite_pool;
    use migrations::run_migrations;
    use repositories::{MangaRepository, ChapterRepository};
    use suwayomi_core::models::{Manga, Chapter, MangaStatus};

    #[tokio::test]
    async fn test_db_operations() {
        let pool = create_sqlite_pool("sqlite::memory:").await.expect("Failed to create pool");
        run_migrations(&pool).await.expect("Failed to run migrations");

        let manga_repo = MangaRepository::new(&pool);
        
        let manga = Manga {
            id: 0, // id will be auto-incremented
            source_id: 1,
            url: "/manga/1".to_string(),
            title: "Test Manga".to_string(),
            artist: Some("Test Artist".to_string()),
            author: Some("Test Author".to_string()),
            description: Some("Test Description".to_string()),
            genre: Some(vec!["Action".to_string(), "Adventure".to_string()]),
            status: MangaStatus::Ongoing,
            thumbnail_url: Some("http://example.com/thumb.jpg".to_string()),
            update_strategy: 1,
            initialized: true,
        };

        let manga_id = manga_repo.insert(&manga).await.expect("Failed to insert manga");
        assert!(manga_id > 0);

        let fetched_manga = manga_repo.get_by_id(manga_id).await.expect("Failed to fetch manga").unwrap();
        assert_eq!(fetched_manga.title, "Test Manga");
        assert_eq!(fetched_manga.url, "/manga/1");
        assert_eq!(fetched_manga.genre.unwrap(), vec!["Action".to_string(), "Adventure".to_string()]);

        let chapter_repo = ChapterRepository::new(&pool);

        let chapter = Chapter {
            id: 0,
            manga_id,
            url: "/chapter/1".to_string(),
            name: "Chapter 1".to_string(),
            date_upload: 1234567890,
            chapter_number: 1.0,
            scanlator: Some("Test Scanlator".to_string()),
            read: false,
            bookmark: false,
            last_page_read: 0,
            date_fetch: 1234567890,
            source_order: 1,
        };

        chapter_repo.insert_chapters(&[chapter]).await.expect("Failed to insert chapter");

        let chapters = chapter_repo.get_by_manga_id(manga_id).await.expect("Failed to fetch chapters");
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].name, "Chapter 1");

        let chap_id = chapters[0].id;
        chapter_repo.update_read(chap_id, true, 10).await.expect("Failed to update read status");

        let updated_chap = chapter_repo.get_by_id(chap_id).await.expect("Failed to fetch updated chapter").unwrap();
        assert!(updated_chap.read);
        assert_eq!(updated_chap.last_page_read, 10);
    }
}
