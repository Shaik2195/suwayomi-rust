use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MangaStatus {
    Unknown,
    Ongoing,
    Completed,
    Licensed,
    PublishingFinished,
    Cancelled,
    OnHiatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manga {
    pub id: i64,
    pub source_id: i64,
    pub url: String,
    pub title: String,
    pub artist: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub genre: Option<Vec<String>>,
    pub status: MangaStatus,
    pub thumbnail_url: Option<String>,
    pub update_strategy: i32,
    pub initialized: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Chapter {
    pub id: i64,
    pub manga_id: i64,
    pub url: String,
    pub name: String,
    pub date_upload: i64,
    pub chapter_number: f32,
    pub scanlator: Option<String>,
    pub read: bool,
    pub bookmark: bool,
    pub last_page_read: i64,
    pub date_fetch: i64,
    pub source_order: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub order: i32,
    pub flags: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Extension {
    pub name: String,
    pub pkg_name: String,
    pub version_name: String,
    pub version_code: i64,
    pub lang: String,
    pub is_nsfw: bool,
    pub has_readme: bool,
    pub has_changelog: bool,
    pub is_obsolete: bool,
    pub is_unofficial: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub lang: String,
    pub supports_latest: bool,
    pub is_configured: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Track {
    pub id: i64,
    pub manga_id: i64,
    pub sync_id: i64,
    pub remote_id: i64,
    pub library_id: Option<i64>,
    pub title: String,
    pub last_chapter_read: f32,
    pub total_chapters: i32,
    pub status: i32,
    pub score: f32,
    pub tracking_url: String,
    pub start_date: i64,
    pub finish_date: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Downloaded,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DownloadQueueItem {
    pub chapter_id: i64,
    pub manga_id: i64,
    pub status: DownloadStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PageStatus {
    Queue,
    LoadPage,
    DownloadImage,
    Ready,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Page {
    pub index: i32,
    pub url: String,
    pub image_url: Option<String>,
    pub status: PageStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MangaPage {
    pub manga_list: Vec<Manga>,
    pub has_next_page: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Filter {
    pub name: String,
    pub state: FilterValue,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum FilterValue {
    Header,
    Separator,
    Text(String),
    CheckBox(bool),
    TriState(i32),
    Group(Vec<Filter>),
    Sort(String, bool),
    Select(i32, Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manga_serialization() {
        let manga = Manga {
            id: 1,
            source_id: 1,
            url: "/manga/1".to_string(),
            title: "Test Manga".to_string(),
            artist: Some("Artist".to_string()),
            author: Some("Author".to_string()),
            description: Some("Description".to_string()),
            genre: Some(vec!["Action".to_string()]),
            status: MangaStatus::Ongoing,
            thumbnail_url: Some("http://example.com/thumb.jpg".to_string()),
            update_strategy: 0,
            initialized: true,
        };

        let serialized = serde_json::to_string(&manga).unwrap();
        let deserialized: Manga = serde_json::from_str(&serialized).unwrap();

        assert_eq!(manga, deserialized);
    }

    #[test]
    fn test_chapter_serialization() {
        let chapter = Chapter {
            id: 1,
            manga_id: 1,
            url: "/chapter/1".to_string(),
            name: "Chapter 1".to_string(),
            date_upload: 1234567890,
            chapter_number: 1.0,
            scanlator: Some("Scanlator".to_string()),
            read: false,
            bookmark: false,
            last_page_read: 0,
            date_fetch: 1234567890,
            source_order: 1,
        };

        let serialized = serde_json::to_string(&chapter).unwrap();
        let deserialized: Chapter = serde_json::from_str(&serialized).unwrap();

        assert_eq!(chapter, deserialized);
    }
}
