use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MangaStatus {
    Unknown,
    Ongoing,
    Completed,
    Licensed,
    PublishingFinished,
    Cancelled,
    OnHiatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manga {
    pub id: Option<i64>,
    pub title: String,
    pub url: String,
    pub artist: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub genre: Option<Vec<String>>,
    pub status: MangaStatus,
    pub thumbnail_url: Option<String>,
    pub source_id: i64,
    pub initialized: bool,
    pub in_library: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: Option<i64>,
    pub manga_id: i64,
    pub url: String,
    pub name: String,
    pub scanlator: Option<String>,
    pub read: bool,
    pub bookmark: bool,
    pub last_page_read: i32,
    pub chapter_number: f32,
    pub source_order: i32,
    pub date_fetch: i64,
    pub date_upload: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub order_index: i32,
    pub flags: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extension {
    pub pkg_name: String,
    pub name: String,
    pub version_name: String,
    pub version_code: i32,
    pub lang: String,
    pub is_nsfw: bool,
    pub has_readme: bool,
    pub has_changelog: bool,
    pub is_obsolete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub lang: String,
    pub name: String,
    pub supports_latest: bool,
    pub is_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: Option<i64>,
    pub manga_id: i64,
    pub sync_id: i32,
    pub media_id: i64,
    pub library_id: Option<i64>,
    pub title: String,
    pub last_chapter_read: f32,
    pub total_chapters: i32,
    pub status: i32,
    pub score: f32,
    pub tracking_url: Option<String>,
    pub start_date: i64,
    pub finish_date: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Downloaded,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadQueueItem {
    pub id: Option<i64>,
    pub manga_id: i64,
    pub chapter_id: i64,
    pub order_index: i32,
    pub status: DownloadStatus,
    pub progress: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageStatus {
    Queue,
    Download,
    Ready,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub index: u32,
    pub url: String,
    pub image_url: Option<String>,
    pub status: PageStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MangaPage {
    pub mangas: Vec<Manga>,
    pub has_next_page: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriState {
    Ignore,
    Include,
    Exclude,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortState {
    pub index: usize,
    pub ascending: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum Filter {
    Text {
        name: String,
        state: String,
    },
    Checkbox {
        name: String,
        state: bool,
    },
    TriState {
        name: String,
        state: TriState,
    },
    Select {
        name: String,
        values: Vec<String>,
        state: usize,
    },
    Sort {
        name: String,
        values: Vec<String>,
        state: Option<SortState>,
    },
}
