#[cfg(test)]
mod tests {
    use crate::error::{Result, SuwayomiError};
    use crate::models::{
        Category, Chapter, DownloadQueueItem, DownloadStatus, Extension, Filter, Manga, MangaPage,
        MangaStatus, Page, PageStatus, SortState, Source, Track, TriState,
    };
    use crate::traits::MangaSource;
    use async_trait::async_trait;

    #[test]
    fn test_manga_serde() {
        let manga = Manga {
            id: Some(1),
            title: "One Piece".to_string(),
            url: "/manga/one-piece".to_string(),
            artist: Some("Eiichiro Oda".to_string()),
            author: Some("Eiichiro Oda".to_string()),
            description: Some("Pirate adventure".to_string()),
            genre: Some(vec!["Action".to_string(), "Adventure".to_string()]),
            status: MangaStatus::Ongoing,
            thumbnail_url: Some("https://example.com/thumb.jpg".to_string()),
            source_id: 100,
            initialized: true,
            in_library: true,
        };

        let json = serde_json::to_string(&manga).unwrap();
        let deserialized: Manga = serde_json::from_str(&json).unwrap();
        assert_eq!(manga, deserialized);
    }

    #[test]
    fn test_chapter_serde() {
        let chapter = Chapter {
            id: Some(10),
            manga_id: 1,
            url: "/chapter/1".to_string(),
            name: "Chapter 1".to_string(),
            scanlator: Some("ScanGroup".to_string()),
            read: false,
            bookmark: true,
            last_page_read: 0,
            chapter_number: 1.0,
            source_order: 1,
            date_fetch: 1600000000,
            date_upload: 1600000000,
        };

        let json = serde_json::to_string(&chapter).unwrap();
        let deserialized: Chapter = serde_json::from_str(&json).unwrap();
        assert_eq!(chapter, deserialized);
    }

    #[test]
    fn test_category_serde() {
        let category = Category {
            id: Some(2),
            name: "Favorites".to_string(),
            order_index: 0,
            flags: 0,
        };

        let json = serde_json::to_string(&category).unwrap();
        let deserialized: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(category, deserialized);
    }

    #[test]
    fn test_extension_serde() {
        let extension = Extension {
            pkg_name: "eu.kanade.tachiyomi.extension.en.mangadex".to_string(),
            name: "MangaDex".to_string(),
            version_name: "1.4.1".to_string(),
            version_code: 14,
            lang: "en".to_string(),
            is_nsfw: false,
            has_readme: true,
            has_changelog: false,
            is_obsolete: false,
        };

        let json = serde_json::to_string(&extension).unwrap();
        let deserialized: Extension = serde_json::from_str(&json).unwrap();
        assert_eq!(extension, deserialized);
    }

    #[test]
    fn test_source_serde() {
        let source = Source {
            id: 100,
            lang: "en".to_string(),
            name: "MangaDex".to_string(),
            supports_latest: true,
            is_configured: true,
        };

        let json = serde_json::to_string(&source).unwrap();
        let deserialized: Source = serde_json::from_str(&json).unwrap();
        assert_eq!(source, deserialized);
    }

    #[test]
    fn test_track_serde() {
        let track = Track {
            id: Some(5),
            manga_id: 1,
            sync_id: 1,
            media_id: 12345,
            library_id: Some(10),
            title: "One Piece".to_string(),
            last_chapter_read: 1000.0,
            total_chapters: 0,
            status: 1,
            score: 9.5,
            tracking_url: Some("https://myanimelist.net/manga/13".to_string()),
            start_date: 1500000000,
            finish_date: 0,
        };

        let json = serde_json::to_string(&track).unwrap();
        let deserialized: Track = serde_json::from_str(&json).unwrap();
        assert_eq!(track, deserialized);
    }

    #[test]
    fn test_download_queue_item_serde() {
        let item = DownloadQueueItem {
            id: Some(1),
            manga_id: 1,
            chapter_id: 10,
            order_index: 0,
            status: DownloadStatus::Downloading,
            progress: 50.5,
        };

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: DownloadQueueItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_page_serde() {
        let page = Page {
            index: 0,
            url: "https://example.com/page1.html".to_string(),
            image_url: Some("https://example.com/page1.jpg".to_string()),
            status: PageStatus::Ready,
        };

        let json = serde_json::to_string(&page).unwrap();
        let deserialized: Page = serde_json::from_str(&json).unwrap();
        assert_eq!(page, deserialized);
    }

    #[test]
    fn test_filter_serde() {
        let filters = vec![
            Filter::Text {
                name: "Search".to_string(),
                state: "One Piece".to_string(),
            },
            Filter::Checkbox {
                name: "Completed".to_string(),
                state: true,
            },
            Filter::TriState {
                name: "Licensed".to_string(),
                state: TriState::Include,
            },
            Filter::Select {
                name: "Genre".to_string(),
                values: vec!["Action".to_string(), "Comedy".to_string()],
                state: 0,
            },
            Filter::Sort {
                name: "Sort By".to_string(),
                values: vec!["Title".to_string(), "Date".to_string()],
                state: Some(SortState {
                    index: 0,
                    ascending: true,
                }),
            },
        ];

        let json = serde_json::to_string(&filters).unwrap();
        let deserialized: Vec<Filter> = serde_json::from_str(&json).unwrap();
        assert_eq!(filters, deserialized);
    }

    #[test]
    fn test_error_display() {
        let err = SuwayomiError::NotFound("Manga not found".to_string());
        assert_eq!(err.to_string(), "Not Found: Manga not found");

        let err = SuwayomiError::Database("DB connection failed".to_string());
        assert_eq!(err.to_string(), "Database error: DB connection failed");
    }

    struct DummySource;

    #[async_trait]
    impl MangaSource for DummySource {
        async fn get_popular_manga(&self, _page: u32) -> Result<MangaPage> {
            Ok(MangaPage {
                mangas: vec![],
                has_next_page: false,
            })
        }

        async fn get_latest_updates(&self, _page: u32) -> Result<MangaPage> {
            Ok(MangaPage {
                mangas: vec![],
                has_next_page: false,
            })
        }

        async fn search_manga(
            &self,
            _query: &str,
            _page: u32,
            _filters: &[Filter],
        ) -> Result<MangaPage> {
            Ok(MangaPage {
                mangas: vec![],
                has_next_page: false,
            })
        }

        async fn get_manga_details(&self, manga: &Manga) -> Result<Manga> {
            Ok(manga.clone())
        }

        async fn get_chapter_list(&self, _manga: &Manga) -> Result<Vec<Chapter>> {
            Ok(vec![])
        }

        async fn get_page_list(&self, _chapter: &Chapter) -> Result<Vec<Page>> {
            Ok(vec![])
        }

        fn get_filters(&self) -> Vec<Filter> {
            vec![]
        }
    }

    #[tokio::test]
    async fn test_manga_source_trait() {
        let source = DummySource;
        let page = source.get_popular_manga(1).await.unwrap();
        assert!(page.mangas.is_empty());
        assert!(!page.has_next_page);
        assert!(source.get_filters().is_empty());
    }
}
