#![allow(clippy::too_many_arguments)]

use async_graphql::{Context, EmptySubscription, Enum, InputObject, Object, Schema, SimpleObject};
use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::{
    extract::{FromRequestParts, State, WebSocketUpgrade},
    response::{Html, IntoResponse},
};
use sqlx::SqlitePool;

use suwayomi_core::models;
use suwayomi_db::repositories::{CategoryRepository, ChapterRepository, MangaRepository};

// --- Custom Scalars ---

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongString(pub String);

impl From<i64> for LongString {
    fn from(val: i64) -> Self {
        LongString(val.to_string())
    }
}

impl From<String> for LongString {
    fn from(s: String) -> Self {
        LongString(s)
    }
}

impl From<&str> for LongString {
    fn from(s: &str) -> Self {
        LongString(s.to_string())
    }
}

#[async_graphql::Scalar]
impl async_graphql::ScalarType for LongString {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(s) => Ok(LongString(s)),
            async_graphql::Value::Number(n) => Ok(LongString(n.to_string())),
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cursor(pub String);

#[async_graphql::Scalar]
impl async_graphql::ScalarType for Cursor {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        match value {
            async_graphql::Value::String(s) => Ok(Cursor(s)),
            async_graphql::Value::Number(n) => Ok(Cursor(n.to_string())),
            _ => Err(async_graphql::InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.clone())
    }
}

// --- Common Pagination & Enums ---

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "PageInfo")]
pub struct PageInfo {
    #[graphql(name = "endCursor")]
    pub end_cursor: Option<Cursor>,
    #[graphql(name = "hasNextPage")]
    pub has_next_page: bool,
    #[graphql(name = "hasPreviousPage")]
    pub has_previous_page: bool,
    #[graphql(name = "startCursor")]
    pub start_cursor: Option<Cursor>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum SortOrder {
    #[graphql(name = "ASC")]
    Asc,
    #[graphql(name = "DESC")]
    Desc,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum IncludeOrExclude {
    #[graphql(name = "INCLUDE")]
    Include,
    #[graphql(name = "EXCLUDE")]
    Exclude,
}

#[derive(InputObject, Default, Clone)]
pub struct IntFilterInput {
    #[graphql(name = "equalTo")]
    pub equal_to: Option<i32>,
    #[graphql(name = "greaterThan")]
    pub greater_than: Option<i32>,
    #[graphql(name = "lessThan")]
    pub less_than: Option<i32>,
}

#[derive(InputObject, Default, Clone)]
pub struct StringFilterInput {
    #[graphql(name = "equalTo")]
    pub equal_to: Option<String>,
    pub includes: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct BooleanFilterInput {
    #[graphql(name = "equalTo")]
    pub equal_to: Option<bool>,
}

#[derive(InputObject, Default, Clone)]
pub struct LongFilterInput {
    #[graphql(name = "equalTo")]
    pub equal_to: Option<LongString>,
    #[graphql(name = "notEqualTo")]
    pub not_equal_to: Option<LongString>,
    #[graphql(name = "notEqualToAll")]
    pub not_equal_to_all: Option<Vec<LongString>>,
    #[graphql(name = "notEqualToAny")]
    pub not_equal_to_any: Option<Vec<LongString>>,
    #[graphql(name = "greaterThan")]
    pub greater_than: Option<LongString>,
    #[graphql(name = "greaterThanOrEqualTo")]
    pub greater_than_or_equal_to: Option<LongString>,
    #[graphql(name = "lessThan")]
    pub less_than: Option<LongString>,
    #[graphql(name = "lessThanOrEqualTo")]
    pub less_than_or_equal_to: Option<LongString>,
    #[graphql(name = "in")]
    pub in_list: Option<Vec<LongString>>,
    #[graphql(name = "notIn")]
    pub not_in_list: Option<Vec<LongString>>,
    #[graphql(name = "isNull")]
    pub is_null: Option<bool>,
    #[graphql(name = "distinctFrom")]
    pub distinct_from: Option<LongString>,
    #[graphql(name = "distinctFromAll")]
    pub distinct_from_all: Option<Vec<LongString>>,
    #[graphql(name = "distinctFromAny")]
    pub distinct_from_any: Option<Vec<LongString>>,
    #[graphql(name = "notDistinctFrom")]
    pub not_distinct_from: Option<LongString>,
}

// --- Categories ---

#[derive(Enum, Copy, Clone, Eq, PartialEq, Default)]
pub enum CategoryOrderBy {
    #[graphql(name = "ID")]
    Id,
    #[graphql(name = "NAME")]
    Name,
    #[default]
    #[graphql(name = "ORDER")]
    Order,
}

#[derive(InputObject, Default, Clone)]
pub struct CategoryOrderInput {
    pub by: CategoryOrderBy,
    #[graphql(name = "byType")]
    pub by_type: Option<SortOrder>,
}

#[derive(InputObject, Default, Clone)]
pub struct CategoryConditionInput {
    pub default: Option<bool>,
    pub id: Option<i64>,
    pub name: Option<String>,
    pub order: Option<i32>,
}

#[derive(InputObject, Default, Clone)]
pub struct CategoryFilterInput {
    pub id: Option<IntFilterInput>,
    pub name: Option<StringFilterInput>,
    pub order: Option<IntFilterInput>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryMetaType")]
pub struct CategoryMetaType {
    #[graphql(name = "categoryId")]
    pub category_id: i64,
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "CategoryType")]
pub struct CategoryType {
    pub id: i64,
    pub name: String,
    pub default: bool,
    pub order: i32,
    #[graphql(name = "includeInDownload")]
    pub include_in_download: IncludeOrExclude,
    #[graphql(name = "includeInUpdate")]
    pub include_in_update: IncludeOrExclude,
    #[graphql(name = "isDefaultCategory")]
    pub is_default_category: bool,
    pub meta: Vec<CategoryMetaType>,
    pub mangas: MangaNodeList,
}

impl CategoryType {
    pub fn from_model(cat: models::Category, manga_count: i32) -> Self {
        Self {
            id: cat.id,
            name: cat.name,
            default: cat.id == 1,
            order: cat.order,
            include_in_download: IncludeOrExclude::Include,
            include_in_update: IncludeOrExclude::Include,
            is_default_category: cat.id == 1,
            meta: vec![],
            mangas: MangaNodeList {
                nodes: vec![],
                page_info: PageInfo::default(),
                total_count: manga_count,
            },
        }
    }
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "CategoryNodeList")]
pub struct CategoryNodeList {
    pub nodes: Vec<CategoryType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

// --- Manga ---

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum MangaStatus {
    Unknown,
    Ongoing,
    Completed,
    Licensed,
    PublishingFinished,
    Cancelled,
    OnHiatus,
}

impl From<models::MangaStatus> for MangaStatus {
    fn from(status: models::MangaStatus) -> Self {
        match status {
            models::MangaStatus::Unknown => MangaStatus::Unknown,
            models::MangaStatus::Ongoing => MangaStatus::Ongoing,
            models::MangaStatus::Completed => MangaStatus::Completed,
            models::MangaStatus::Licensed => MangaStatus::Licensed,
            models::MangaStatus::PublishingFinished => MangaStatus::PublishingFinished,
            models::MangaStatus::Cancelled => MangaStatus::Cancelled,
            models::MangaStatus::OnHiatus => MangaStatus::OnHiatus,
        }
    }
}

impl From<MangaStatus> for models::MangaStatus {
    fn from(status: MangaStatus) -> Self {
        match status {
            MangaStatus::Unknown => models::MangaStatus::Unknown,
            MangaStatus::Ongoing => models::MangaStatus::Ongoing,
            MangaStatus::Completed => models::MangaStatus::Completed,
            MangaStatus::Licensed => models::MangaStatus::Licensed,
            MangaStatus::PublishingFinished => models::MangaStatus::PublishingFinished,
            MangaStatus::Cancelled => models::MangaStatus::Cancelled,
            MangaStatus::OnHiatus => models::MangaStatus::OnHiatus,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq, Default)]
pub enum MangaOrderBy {
    #[graphql(name = "ID")]
    Id,
    #[graphql(name = "TITLE")]
    Title,
    #[default]
    #[graphql(name = "IN_LIBRARY_AT")]
    InLibraryAt,
    #[graphql(name = "LAST_FETCHED_AT")]
    LastFetchedAt,
}

#[derive(InputObject, Default, Clone)]
pub struct MangaOrderInput {
    pub by: MangaOrderBy,
    #[graphql(name = "byType")]
    pub by_type: Option<SortOrder>,
}

#[derive(InputObject, Default, Clone)]
pub struct MangaConditionInput {
    pub artist: Option<String>,
    pub author: Option<String>,
    #[graphql(name = "categoryIds")]
    pub category_ids: Option<Vec<i64>>,
    pub description: Option<String>,
    pub genre: Option<Vec<String>>,
    pub id: Option<i64>,
    #[graphql(name = "inLibrary")]
    pub in_library: Option<bool>,
    pub initialized: Option<bool>,
    #[graphql(name = "sourceId")]
    pub source_id: Option<LongString>,
    pub status: Option<MangaStatus>,
    pub title: Option<String>,
    pub url: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct MangaFilterInput {
    pub id: Option<IntFilterInput>,
    #[graphql(name = "inLibrary")]
    pub in_library: Option<BooleanFilterInput>,
    #[graphql(name = "sourceId")]
    pub source_id: Option<LongFilterInput>,
    pub title: Option<StringFilterInput>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum UpdateStrategy {
    #[graphql(name = "ALWAYS_UPDATE")]
    AlwaysUpdate,
    #[graphql(name = "ONLY_FETCH_ONCE")]
    OnlyFetchOnce,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "MangaMetaType")]
pub struct MangaMetaType {
    #[graphql(name = "mangaId")]
    pub manga_id: i64,
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "TrackRecordType")]
pub struct TrackRecordType {
    pub id: i64,
    #[graphql(name = "mangaId")]
    pub manga_id: i64,
    pub title: String,
    #[graphql(name = "trackerId")]
    pub tracker_id: i32,
    pub status: i32,
    pub score: f32,
    #[graphql(name = "displayScore")]
    pub display_score: Option<String>,
    #[graphql(name = "totalChapters")]
    pub total_chapters: i32,
    #[graphql(name = "lastChapterRead")]
    pub last_chapter_read: f32,
    #[graphql(name = "remoteUrl")]
    pub remote_url: Option<String>,
    #[graphql(name = "remoteId")]
    pub remote_id: Option<LongString>,
    #[graphql(name = "libraryId")]
    pub library_id: Option<LongString>,
    #[graphql(name = "startDate")]
    pub start_date: Option<LongString>,
    #[graphql(name = "finishDate")]
    pub finish_date: Option<LongString>,
    pub private: bool,
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "TrackRecordNodeList")]
pub struct TrackRecordNodeList {
    pub nodes: Vec<TrackRecordType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "MangaType")]
pub struct MangaType {
    pub id: i64,
    #[graphql(name = "sourceId")]
    pub source_id: LongString,
    pub url: String,
    pub title: String,
    pub artist: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub genre: Vec<String>,
    pub status: MangaStatus,
    #[graphql(name = "thumbnailUrl")]
    pub thumbnail_url: Option<String>,
    #[graphql(name = "thumbnailUrlLastFetched")]
    pub thumbnail_url_last_fetched: Option<LongString>,
    #[graphql(name = "updateStrategy")]
    pub update_strategy: UpdateStrategy,
    pub initialized: bool,
    #[graphql(name = "inLibrary")]
    pub in_library: bool,
    #[graphql(name = "inLibraryAt")]
    pub in_library_at: LongString,
    #[graphql(name = "lastFetchedAt")]
    pub last_fetched_at: Option<LongString>,
    #[graphql(name = "unreadCount")]
    pub unread_count: i32,
    #[graphql(name = "downloadCount")]
    pub download_count: i32,
    #[graphql(name = "bookmarkCount")]
    pub bookmark_count: i32,
    #[graphql(name = "hasDuplicateChapters")]
    pub has_duplicate_chapters: bool,
    #[graphql(name = "realUrl")]
    pub real_url: Option<String>,
    pub chapters: ChapterNodeList,
    #[graphql(name = "firstUnreadChapter")]
    pub first_unread_chapter: Option<ChapterType>,
    #[graphql(name = "lastReadChapter")]
    pub last_read_chapter: Option<ChapterType>,
    #[graphql(name = "latestReadChapter")]
    pub latest_read_chapter: Option<ChapterType>,
    #[graphql(name = "latestFetchedChapter")]
    pub latest_fetched_chapter: Option<ChapterType>,
    #[graphql(name = "latestUploadedChapter")]
    pub latest_uploaded_chapter: Option<ChapterType>,
    #[graphql(name = "highestNumberedChapter")]
    pub highest_numbered_chapter: Option<ChapterType>,
    #[graphql(name = "chaptersLastFetchedAt")]
    pub chapters_last_fetched_at: Option<LongString>,
    pub age: Option<LongString>,
    #[graphql(name = "chaptersAge")]
    pub chapters_age: Option<LongString>,
    pub categories: CategoryNodeList,
    #[graphql(name = "trackRecords")]
    pub track_records: TrackRecordNodeList,
    pub meta: Vec<MangaMetaType>,
    pub source: Option<SourceType>,
}

impl MangaType {
    pub fn from_model(manga: models::Manga, chapters: Vec<models::Chapter>) -> Self {
        let first_unread = chapters
            .iter()
            .find(|c| !c.read)
            .cloned()
            .map(ChapterType::from);
        let last_read = chapters
            .iter()
            .filter(|c| c.read)
            .next_back()
            .cloned()
            .map(ChapterType::from);
        let latest_uploaded = chapters.first().cloned().map(ChapterType::from);
        let highest_num = chapters
            .iter()
            .max_by(|a, b| {
                a.chapter_number
                    .partial_cmp(&b.chapter_number)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .map(ChapterType::from);

        let unread_count = chapters.iter().filter(|c| !c.read).count() as i32;
        let total_count = chapters.len() as i32;
        let chapter_nodes: Vec<ChapterType> = chapters.into_iter().map(ChapterType::from).collect();

        Self {
            id: manga.id,
            source_id: manga.source_id.into(),
            url: manga.url.clone(),
            title: manga.title,
            artist: manga.artist,
            author: manga.author,
            description: manga.description,
            genre: manga.genre.unwrap_or_default(),
            status: manga.status.into(),
            thumbnail_url: manga.thumbnail_url,
            thumbnail_url_last_fetched: None,
            update_strategy: UpdateStrategy::AlwaysUpdate,
            initialized: manga.initialized,
            in_library: manga.initialized,
            in_library_at: "0".into(),
            last_fetched_at: None,
            unread_count,
            download_count: 0,
            bookmark_count: 0,
            has_duplicate_chapters: false,
            real_url: Some(manga.url),
            chapters: ChapterNodeList {
                nodes: chapter_nodes,
                page_info: PageInfo::default(),
                total_count,
            },
            first_unread_chapter: first_unread,
            last_read_chapter: last_read.clone(),
            latest_read_chapter: last_read,
            latest_fetched_chapter: latest_uploaded.clone(),
            latest_uploaded_chapter: latest_uploaded,
            highest_numbered_chapter: highest_num,
            chapters_last_fetched_at: None,
            age: None,
            chapters_age: None,
            categories: CategoryNodeList::default(),
            track_records: TrackRecordNodeList::default(),
            meta: vec![],
            source: None,
        }
    }
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "MangaNodeList")]
pub struct MangaNodeList {
    pub nodes: Vec<MangaType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

// Backward-compatibility alias for Manga
pub type Manga = MangaType;

// --- Chapters ---

#[derive(Enum, Copy, Clone, Eq, PartialEq, Default)]
pub enum ChapterOrderBy {
    #[graphql(name = "ID")]
    Id,
    #[default]
    #[graphql(name = "SOURCE_ORDER")]
    SourceOrder,
    #[graphql(name = "NAME")]
    Name,
    #[graphql(name = "CHAPTER_NUMBER")]
    ChapterNumber,
    #[graphql(name = "UPLOAD_DATE")]
    UploadDate,
    #[graphql(name = "LAST_READ_AT")]
    LastReadAt,
    #[graphql(name = "FETCHED_AT")]
    FetchedAt,
}

#[derive(InputObject, Default, Clone)]
pub struct ChapterOrderInput {
    pub by: ChapterOrderBy,
    #[graphql(name = "byType")]
    pub by_type: Option<SortOrder>,
}

#[derive(InputObject, Default, Clone)]
pub struct ChapterConditionInput {
    pub id: Option<i64>,
    #[graphql(name = "mangaId")]
    pub manga_id: Option<i64>,
    #[graphql(name = "isRead")]
    pub is_read: Option<bool>,
    #[graphql(name = "isBookmarked")]
    pub is_bookmarked: Option<bool>,
}

#[derive(InputObject, Default, Clone)]
pub struct ChapterFilterInput {
    pub id: Option<IntFilterInput>,
    #[graphql(name = "mangaId")]
    pub manga_id: Option<IntFilterInput>,
    #[graphql(name = "inLibrary")]
    pub in_library: Option<BooleanFilterInput>,
    #[graphql(name = "isRead")]
    pub is_read: Option<BooleanFilterInput>,
    #[graphql(name = "isBookmarked")]
    pub is_bookmarked: Option<BooleanFilterInput>,
    #[graphql(name = "isDownloaded")]
    pub is_downloaded: Option<BooleanFilterInput>,
    #[graphql(name = "lastReadAt")]
    pub last_read_at: Option<LongFilterInput>,
    #[graphql(name = "fetchedAt")]
    pub fetched_at: Option<LongFilterInput>,
    #[graphql(name = "uploadDate")]
    pub upload_date: Option<LongFilterInput>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ChapterMetaType")]
pub struct ChapterMetaType {
    #[graphql(name = "chapterId")]
    pub chapter_id: i64,
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ChapterType")]
pub struct ChapterType {
    pub id: i64,
    #[graphql(name = "mangaId")]
    pub manga_id: i64,
    pub url: String,
    pub name: String,
    #[graphql(name = "uploadDate")]
    pub upload_date: LongString,
    #[graphql(name = "chapterNumber")]
    pub chapter_number: f32,
    pub scanlator: Option<String>,
    #[graphql(name = "isRead")]
    pub is_read: bool,
    #[graphql(name = "isBookmarked")]
    pub is_bookmarked: bool,
    #[graphql(name = "lastPageRead")]
    pub last_page_read: i64,
    #[graphql(name = "lastReadAt")]
    pub last_read_at: LongString,
    #[graphql(name = "fetchedAt")]
    pub fetched_at: LongString,
    #[graphql(name = "sourceOrder")]
    pub source_order: i64,
    #[graphql(name = "pageCount")]
    pub page_count: i32,
    #[graphql(name = "isDownloaded")]
    pub is_downloaded: bool,
    #[graphql(name = "realUrl")]
    pub real_url: Option<String>,
    pub manga: Option<Box<MangaType>>,
    pub meta: Vec<ChapterMetaType>,
}

impl From<models::Chapter> for ChapterType {
    fn from(chapter: models::Chapter) -> Self {
        Self {
            id: chapter.id,
            manga_id: chapter.manga_id,
            url: chapter.url.clone(),
            name: chapter.name,
            upload_date: chapter.date_upload.into(),
            chapter_number: chapter.chapter_number,
            scanlator: chapter.scanlator,
            is_read: chapter.read,
            is_bookmarked: chapter.bookmark,
            last_page_read: chapter.last_page_read,
            last_read_at: "0".into(),
            fetched_at: chapter.date_fetch.into(),
            source_order: chapter.source_order,
            page_count: 0,
            is_downloaded: false,
            real_url: Some(chapter.url),
            manga: None,
            meta: vec![],
        }
    }
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "ChapterNodeList")]
pub struct ChapterNodeList {
    pub nodes: Vec<ChapterType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

// Backward-compatibility alias for Chapter
pub type Chapter = ChapterType;

// --- Sources & Extensions ---

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ContentWarning {
    #[graphql(name = "SAFE")]
    Safe,
    #[graphql(name = "SUGGESTIVE")]
    Suggestive,
    #[graphql(name = "NSFW")]
    Nsfw,
}

#[derive(InputObject, Default, Clone)]
pub struct SourceOrderInput {
    pub by: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct SourceConditionInput {
    pub lang: Option<String>,
    pub id: Option<LongString>,
}

#[derive(InputObject, Default, Clone)]
pub struct SourceFilterInput {
    pub lang: Option<StringFilterInput>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "SourceMetaType")]
pub struct SourceMetaType {
    #[graphql(name = "sourceId")]
    pub source_id: LongString,
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "SourceType")]
pub struct SourceType {
    pub id: LongString,
    pub name: String,
    #[graphql(name = "displayName")]
    pub display_name: String,
    pub lang: String,
    #[graphql(name = "iconUrl")]
    pub icon_url: String,
    #[graphql(name = "contentWarning")]
    pub content_warning: ContentWarning,
    #[graphql(name = "isConfigurable")]
    pub is_configurable: bool,
    #[graphql(name = "isNsfw")]
    pub is_nsfw: bool,
    #[graphql(name = "supportsLatest")]
    pub supports_latest: bool,
    #[graphql(name = "baseUrl")]
    pub base_url: Option<String>,
    #[graphql(name = "homeUrl")]
    pub home_url: Option<String>,
    pub extension: Option<Box<ExtensionType>>,
    pub meta: Vec<SourceMetaType>,
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "SourceNodeList")]
pub struct SourceNodeList {
    pub nodes: Vec<SourceType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

// Backward-compatibility alias for Source
pub type Source = SourceType;

#[derive(InputObject, Default, Clone)]
pub struct ExtensionOrderInput {
    pub by: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct ExtensionConditionInput {
    pub lang: Option<String>,
    #[graphql(name = "pkgName")]
    pub pkg_name: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct ExtensionFilterInput {
    pub lang: Option<StringFilterInput>,
}

#[derive(InputObject, Default, Clone)]
pub struct ExtensionStoreConditionInput {
    pub id: Option<LongString>,
    #[graphql(name = "indexUrl")]
    pub index_url: Option<String>,
    pub name: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct ExtensionStoreFilterInput {
    #[graphql(name = "indexUrl")]
    pub index_url: Option<StringFilterInput>,
    pub name: Option<StringFilterInput>,
}

#[derive(InputObject, Default, Clone)]
pub struct ExtensionStoreOrderInput {
    pub by: Option<String>,
    #[graphql(name = "byType")]
    pub by_type: Option<SortOrder>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ExtensionType")]
pub struct ExtensionType {
    #[graphql(name = "pkgName")]
    pub pkg_name: String,
    pub name: String,
    pub lang: String,
    #[graphql(name = "versionCodeLong")]
    pub version_code_long: LongString,
    #[graphql(name = "versionCode")]
    pub version_code: i32,
    #[graphql(name = "versionName")]
    pub version_name: String,
    #[graphql(name = "iconUrl")]
    pub icon_url: String,
    #[graphql(name = "apkUrl")]
    pub apk_url: Option<String>,
    #[graphql(name = "apkName")]
    pub apk_name: Option<String>,
    #[graphql(name = "jarUrl")]
    pub jar_url: Option<String>,
    #[graphql(name = "repo")]
    pub repo: Option<String>,
    #[graphql(name = "storeIndexUrl")]
    pub store_index_url: Option<String>,
    #[graphql(name = "contentWarning")]
    pub content_warning: ContentWarning,
    #[graphql(name = "isInstalled")]
    pub is_installed: bool,
    #[graphql(name = "isNsfw")]
    pub is_nsfw: bool,
    #[graphql(name = "isObsolete")]
    pub is_obsolete: bool,
    #[graphql(name = "hasUpdate")]
    pub has_update: bool,
    #[graphql(name = "extensionLib")]
    pub extension_lib: Option<String>,
    #[graphql(name = "extensionStore")]
    pub extension_store: Option<Box<ExtensionStoreType>>,
    pub source: SourceNodeList,
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "ExtensionNodeList")]
pub struct ExtensionNodeList {
    pub nodes: Vec<ExtensionType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "ExtensionStoreType")]
pub struct ExtensionStoreType {
    #[graphql(name = "badgeLabel")]
    pub badge_label: String,
    #[graphql(name = "contactDiscord")]
    pub contact_discord: Option<String>,
    #[graphql(name = "contactWebsite")]
    pub contact_website: String,
    #[graphql(name = "extensionListUrl")]
    pub extension_list_url: Option<String>,
    #[graphql(name = "indexUrl")]
    pub index_url: String,
    #[graphql(name = "isLegacy")]
    pub is_legacy: bool,
    pub name: String,
    #[graphql(name = "signingKey")]
    pub signing_key: String,
    pub extensions: ExtensionNodeList,
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "ExtensionStoreNodeList")]
pub struct ExtensionStoreNodeList {
    pub nodes: Vec<ExtensionStoreType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

// Backward-compatibility listing types
#[derive(SimpleObject, Clone)]
pub struct ExtensionListing {
    #[graphql(name = "pkgName")]
    pub pkg_name: String,
    pub name: String,
    #[graphql(name = "versionName")]
    pub version_name: String,
    #[graphql(name = "versionCode")]
    pub version_code: i64,
    pub lang: String,
    #[graphql(name = "isNsfw")]
    pub is_nsfw: bool,
    #[graphql(name = "apkUrl")]
    pub apk_url: String,
    #[graphql(name = "iconUrl")]
    pub icon_url: String,
}

impl From<suwayomi_extensions::types::ExtensionListing> for ExtensionListing {
    fn from(listing: suwayomi_extensions::types::ExtensionListing) -> Self {
        Self {
            pkg_name: listing.pkg_name,
            name: listing.name,
            version_name: listing.version_name,
            version_code: listing.version_code,
            lang: listing.lang,
            is_nsfw: listing.is_nsfw,
            apk_url: listing.apk_url,
            icon_url: listing.icon_url,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct ExtensionRepo {
    pub name: String,
    pub url: String,
}

impl From<suwayomi_extensions::types::ExtensionRepo> for ExtensionRepo {
    fn from(repo: suwayomi_extensions::types::ExtensionRepo) -> Self {
        Self {
            name: repo.name,
            url: repo.url,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Page {
    pub index: i32,
    pub url: String,
    #[graphql(name = "imageUrl")]
    pub image_url: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct MangaPagePayload {
    pub mangas: Vec<MangaType>,
    #[graphql(name = "hasNextPage")]
    pub has_next_page: bool,
}

// --- Platform & Server Info ---

#[derive(SimpleObject, Clone)]
pub struct PlatformInfo {
    pub arch: String,
    pub headless: bool,
    pub os: OsInfo,
    pub jvm: JvmInfo,
}

#[derive(SimpleObject, Clone)]
pub struct OsInfo {
    pub name: String,
    pub build: String,
    pub version: String,
}

#[derive(SimpleObject, Clone)]
pub struct JvmInfo {
    #[graphql(name = "javaVersion")]
    pub java_version: String,
    #[graphql(name = "vmName")]
    pub vm_name: String,
    #[graphql(name = "vmVendor")]
    pub vm_vendor: String,
    #[graphql(name = "vmVersion")]
    pub vm_version: String,
}

#[derive(SimpleObject)]
pub struct AboutServerPayload {
    pub name: String,
    pub version: String,
    #[graphql(name = "buildTime")]
    pub build_time: String,
    #[graphql(name = "buildType")]
    pub build_type: String,
    pub discord: String,
    pub github: String,
    #[graphql(name = "platformInfo")]
    pub platform_info: PlatformInfo,
}

#[derive(SimpleObject)]
pub struct AboutWebUI {
    pub channel: String,
    pub tag: String,
    #[graphql(name = "updateTimestamp")]
    pub update_timestamp: String,
}

// --- Settings & Downloader ---

#[derive(SimpleObject, Clone)]
#[graphql(name = "SettingsDownloadConversionHeader")]
pub struct SettingsDownloadConversionHeader {
    pub name: String,
    pub value: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "SettingsDownloadConversion")]
pub struct SettingsDownloadConversion {
    #[graphql(name = "mimeType")]
    pub mime_type: String,
    pub target: String,
    #[graphql(name = "compressionLevel")]
    pub compression_level: i32,
    pub headers: Vec<SettingsDownloadConversionHeader>,
    #[graphql(name = "callTimeout")]
    pub call_timeout: i32,
    #[graphql(name = "connectTimeout")]
    pub connect_timeout: i32,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "SettingsType")]
pub struct SettingsType {
    pub ip: String,
    pub port: i32,
    #[graphql(name = "socksProxyEnabled")]
    pub socks_proxy_enabled: bool,
    #[graphql(name = "socksProxyVersion")]
    pub socks_proxy_version: i32,
    #[graphql(name = "socksProxyHost")]
    pub socks_proxy_host: String,
    #[graphql(name = "socksProxyPort")]
    pub socks_proxy_port: i32,
    #[graphql(name = "socksProxyUsername")]
    pub socks_proxy_username: String,
    #[graphql(name = "socksProxyPassword")]
    pub socks_proxy_password: String,
    #[graphql(name = "webUIFlavor")]
    pub web_ui_flavor: String,
    #[graphql(name = "initialOpenInBrowserEnabled")]
    pub initial_open_in_browser_enabled: bool,
    #[graphql(name = "webUIInterface")]
    pub web_ui_interface: String,
    #[graphql(name = "electronPath")]
    pub electron_path: String,
    #[graphql(name = "webUIChannel")]
    pub web_ui_channel: String,
    #[graphql(name = "webUIUpdateCheckInterval")]
    pub web_ui_update_check_interval: i32,
    #[graphql(name = "downloadAsCbz")]
    pub download_as_cbz: bool,
    #[graphql(name = "downloadsPath")]
    pub downloads_path: String,
    #[graphql(name = "autoDownloadNewChapters")]
    pub auto_download_new_chapters: bool,
    #[graphql(name = "excludeEntryWithUnreadChapters")]
    pub exclude_entry_with_unread_chapters: bool,
    #[graphql(name = "autoDownloadNewChaptersLimit")]
    pub auto_download_new_chapters_limit: i32,
    #[graphql(name = "autoDownloadIgnoreReUploads")]
    pub auto_download_ignore_re_uploads: bool,
    #[graphql(name = "downloadConversions")]
    pub download_conversions: Vec<SettingsDownloadConversion>,
    #[graphql(name = "serveConversions")]
    pub serve_conversions: Vec<SettingsDownloadConversion>,
    #[graphql(name = "maxSourcesInParallel")]
    pub max_sources_in_parallel: i32,
    #[graphql(name = "excludeUnreadChapters")]
    pub exclude_unread_chapters: bool,
    #[graphql(name = "excludeNotStarted")]
    pub exclude_not_started: bool,
    #[graphql(name = "excludeCompleted")]
    pub exclude_completed: bool,
    #[graphql(name = "globalUpdateInterval")]
    pub global_update_interval: i32,
    #[graphql(name = "updateMangas")]
    pub update_mangas: bool,
    #[graphql(name = "authMode")]
    pub auth_mode: String,
    #[graphql(name = "authPassword")]
    pub auth_password: String,
    #[graphql(name = "authUsername")]
    pub auth_username: String,
    #[graphql(name = "jwtAudience")]
    pub jwt_audience: String,
    #[graphql(name = "jwtTokenExpiry")]
    pub jwt_token_expiry: i32,
    #[graphql(name = "jwtRefreshExpiry")]
    pub jwt_refresh_expiry: i32,
    #[graphql(name = "debugLogsEnabled")]
    pub debug_logs_enabled: bool,
    #[graphql(name = "systemTrayEnabled")]
    pub system_tray_enabled: bool,
    #[graphql(name = "maxLogFileSize")]
    pub max_log_file_size: i32,
    #[graphql(name = "maxLogFiles")]
    pub max_log_files: i32,
    #[graphql(name = "maxLogFolderSize")]
    pub max_log_folder_size: i32,
    #[graphql(name = "backupPath")]
    pub backup_path: String,
    #[graphql(name = "backupTime")]
    pub backup_time: String,
    #[graphql(name = "backupInterval")]
    pub backup_interval: i32,
    #[graphql(name = "backupTTL")]
    pub backup_ttl: i32,
    #[graphql(name = "autoBackupIncludeCategories")]
    pub auto_backup_include_categories: bool,
    #[graphql(name = "autoBackupIncludeChapters")]
    pub auto_backup_include_chapters: bool,
    #[graphql(name = "autoBackupIncludeClientData")]
    pub auto_backup_include_client_data: bool,
    #[graphql(name = "autoBackupIncludeHistory")]
    pub auto_backup_include_history: bool,
    #[graphql(name = "autoBackupIncludeManga")]
    pub auto_backup_include_manga: bool,
    #[graphql(name = "autoBackupIncludeServerSettings")]
    pub auto_backup_include_server_settings: bool,
    #[graphql(name = "autoBackupIncludeTracking")]
    pub auto_backup_include_tracking: bool,
    #[graphql(name = "localSourcePath")]
    pub local_source_path: String,
    #[graphql(name = "flareSolverrEnabled")]
    pub flare_solverr_enabled: bool,
    #[graphql(name = "flareSolverrUrl")]
    pub flare_solverr_url: String,
    #[graphql(name = "flareSolverrTimeout")]
    pub flare_solverr_timeout: i32,
    #[graphql(name = "flareSolverrSessionName")]
    pub flare_solverr_session_name: String,
    #[graphql(name = "flareSolverrSessionTtl")]
    pub flare_solverr_session_ttl: i32,
    #[graphql(name = "flareSolverrAsResponseFallback")]
    pub flare_solverr_as_response_fallback: bool,
    #[graphql(name = "opdsUseBinaryFileSizes")]
    pub opds_use_binary_file_sizes: bool,
    #[graphql(name = "opdsItemsPerPage")]
    pub opds_items_per_page: i32,
    #[graphql(name = "opdsEnablePageReadProgress")]
    pub opds_enable_page_read_progress: bool,
    #[graphql(name = "opdsMarkAsReadOnDownload")]
    pub opds_mark_as_read_on_download: bool,
    #[graphql(name = "opdsShowOnlyUnreadChapters")]
    pub opds_show_only_unread_chapters: bool,
    #[graphql(name = "opdsShowOnlyDownloadedChapters")]
    pub opds_show_only_downloaded_chapters: bool,
    #[graphql(name = "opdsChapterSortOrder")]
    pub opds_chapter_sort_order: String,
    #[graphql(name = "opdsCbzMimetype")]
    pub opds_cbz_mimetype: String,
    #[graphql(name = "opdsSkipChapterMetadataFeed")]
    pub opds_skip_chapter_metadata_feed: bool,
    #[graphql(name = "koreaderSyncChecksumMethod")]
    pub koreader_sync_checksum_method: String,
    #[graphql(name = "koreaderSyncStrategyBackward")]
    pub koreader_sync_strategy_backward: String,
    #[graphql(name = "koreaderSyncStrategyForward")]
    pub koreader_sync_strategy_forward: String,
    #[graphql(name = "koreaderSyncPercentageTolerance")]
    pub koreader_sync_percentage_tolerance: i32,
    #[graphql(name = "databaseType")]
    pub database_type: String,
    #[graphql(name = "databaseUrl")]
    pub database_url: String,
    #[graphql(name = "databaseUsername")]
    pub database_username: String,
    #[graphql(name = "databasePassword")]
    pub database_password: String,
    #[graphql(name = "useHikariConnectionPool")]
    pub use_hikari_connection_pool: bool,
    #[graphql(name = "kcefEnabled")]
    pub kcef_enabled: bool,
    #[graphql(name = "syncDataCategories")]
    pub sync_data_categories: bool,
    #[graphql(name = "syncDataChapters")]
    pub sync_data_chapters: bool,
    #[graphql(name = "syncDataHistory")]
    pub sync_data_history: bool,
    #[graphql(name = "syncDataManga")]
    pub sync_data_manga: bool,
    #[graphql(name = "syncDataTracking")]
    pub sync_data_tracking: bool,
    #[graphql(name = "syncInterval")]
    pub sync_interval: i32,
    #[graphql(name = "syncYomiApiKey")]
    pub sync_yomi_api_key: String,
    #[graphql(name = "syncYomiEnabled")]
    pub sync_yomi_enabled: bool,
    #[graphql(name = "syncYomiHost")]
    pub sync_yomi_host: String,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum DownloaderState {
    #[graphql(name = "STOPPED")]
    Stopped,
    #[graphql(name = "STARTED")]
    Started,
    #[graphql(name = "PAUSED")]
    Paused,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum DownloadState {
    #[graphql(name = "QUEUED")]
    Queued,
    #[graphql(name = "DOWNLOADING")]
    Downloading,
    #[graphql(name = "DOWNLOADED")]
    Downloaded,
    #[graphql(name = "ERROR")]
    Error,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "DownloadType")]
pub struct DownloadType {
    pub chapter: ChapterType,
    pub manga: MangaType,
    pub progress: f32,
    pub state: DownloadState,
    pub tries: i32,
    pub position: i32,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "DownloadStatus")]
pub struct DownloadStatus {
    pub state: DownloaderState,
    pub queue: Vec<DownloadType>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "UpdateStatus")]
pub struct UpdateStatus {
    #[graphql(name = "completeJobs")]
    pub complete_jobs: i32,
    #[graphql(name = "failedJobs")]
    pub failed_jobs: i32,
    #[graphql(name = "isRunning")]
    pub is_running: bool,
    #[graphql(name = "pendingJobs")]
    pub pending_jobs: i32,
    #[graphql(name = "runningJobs")]
    pub running_jobs: i32,
    #[graphql(name = "skippedCategories")]
    pub skipped_categories: i32,
    #[graphql(name = "skippedJobs")]
    pub skipped_jobs: i32,
    #[graphql(name = "updatingCategories")]
    pub updating_categories: Vec<i64>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "WebUIUpdateCheck")]
pub struct WebUIUpdateCheck {
    pub channel: String,
    pub tag: String,
    #[graphql(name = "updateAvailable")]
    pub update_available: bool,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "WebUIUpdateStatus")]
pub struct WebUIUpdateStatus {
    pub info: String,
    pub progress: f32,
    pub state: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "LastUpdateTimestampPayload")]
pub struct LastUpdateTimestampPayload {
    pub timestamp: LongString,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "LibraryUpdateStatus")]
pub struct LibraryUpdateStatus {
    #[graphql(name = "categoryUpdates")]
    pub category_updates: Vec<String>,
    #[graphql(name = "jobsInfo")]
    pub jobs_info: String,
    #[graphql(name = "mangaUpdates")]
    pub manga_updates: Vec<String>,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "CheckForServerUpdatesPayload")]
pub struct CheckForServerUpdatesPayload {
    pub channel: String,
    pub tag: String,
}

#[derive(SimpleObject, Clone)]
#[graphql(name = "GlobalMetaType")]
pub struct GlobalMetaType {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject, Clone, Default)]
#[graphql(name = "GlobalMetaNodeList")]
pub struct GlobalMetaNodeList {
    pub nodes: Vec<GlobalMetaType>,
    #[graphql(name = "pageInfo")]
    pub page_info: PageInfo,
    #[graphql(name = "totalCount")]
    pub total_count: i32,
}

// --- Query Root ---

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    #[graphql(name = "aboutServer")]
    async fn about_server(&self) -> async_graphql::Result<AboutServerPayload> {
        Ok(AboutServerPayload {
            name: "Suwayomi-Rust".to_string(),
            version: "0.1.0".to_string(),
            build_time: "1790737255000".to_string(),
            build_type: "release".to_string(),
            discord: "".to_string(),
            github: "https://github.com/Shaik2195/suwayomi-rust".to_string(),
            platform_info: PlatformInfo {
                arch: std::env::consts::ARCH.to_string(),
                headless: false,
                os: OsInfo {
                    name: std::env::consts::OS.to_string(),
                    build: "unknown".to_string(),
                    version: "unknown".to_string(),
                },
                jvm: JvmInfo {
                    java_version: "21.0.0 (Native)".to_string(),
                    vm_name: "Suwayomi-Rust Tokio Engine".to_string(),
                    vm_vendor: "Suwayomi-Rust".to_string(),
                    vm_version: "0.1.0".to_string(),
                },
            },
        })
    }

    #[graphql(name = "aboutWebUI")]
    async fn about_web_ui(&self) -> async_graphql::Result<AboutWebUI> {
        Ok(AboutWebUI {
            channel: "stable".to_string(),
            tag: "latest".to_string(),
            update_timestamp: "1790737255000".to_string(),
        })
    }

    #[graphql(name = "categories")]
    async fn categories(
        &self,
        ctx: &Context<'_>,
        _condition: Option<CategoryConditionInput>,
        _filter: Option<CategoryFilterInput>,
        _order: Option<Vec<CategoryOrderInput>>,
        _before: Option<Cursor>,
        _after: Option<Cursor>,
        _first: Option<i32>,
        _last: Option<i32>,
        _offset: Option<i32>,
    ) -> async_graphql::Result<CategoryNodeList> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = CategoryRepository::new(pool);
        let mut db_cats = repo.get_categories().await?;
        let manga_repo = MangaRepository::new(pool);
        let library = manga_repo.get_library().await?;
        let total_manga_count = library.len() as i32;

        if db_cats.is_empty() {
            db_cats.push(models::Category {
                id: 1,
                name: "Default".to_string(),
                order: 0,
                flags: 0,
            });
        }

        let nodes: Vec<CategoryType> = db_cats
            .into_iter()
            .map(|c| CategoryType::from_model(c, total_manga_count))
            .collect();

        let count = nodes.len() as i32;
        Ok(CategoryNodeList {
            nodes,
            page_info: PageInfo::default(),
            total_count: count,
        })
    }

    #[graphql(name = "category")]
    async fn category(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> async_graphql::Result<Option<CategoryType>> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = CategoryRepository::new(pool);
        let cat = repo.get_by_id(id).await?;
        let manga_repo = MangaRepository::new(pool);
        let library = manga_repo.get_library().await?;
        let count = library.len() as i32;
        Ok(cat.map(|c| CategoryType::from_model(c, count)))
    }

    #[graphql(name = "mangas")]
    async fn mangas(
        &self,
        ctx: &Context<'_>,
        condition: Option<MangaConditionInput>,
        _filter: Option<MangaFilterInput>,
        _order: Option<Vec<MangaOrderInput>>,
        _before: Option<Cursor>,
        _after: Option<Cursor>,
        _first: Option<i32>,
        _last: Option<i32>,
        _offset: Option<i32>,
    ) -> async_graphql::Result<MangaNodeList> {
        let pool = ctx.data::<SqlitePool>()?;
        let manga_repo = MangaRepository::new(pool);
        let chapter_repo = ChapterRepository::new(pool);

        let raw_mangas = manga_repo.get_library().await?;
        let mut nodes = Vec::new();

        for m in raw_mangas {
            if let Some(ref cond) = condition {
                if let Some(in_lib) = cond.in_library {
                    if in_lib != m.initialized {
                        continue;
                    }
                }
                if let Some(ref sid) = cond.source_id {
                    if sid.0 != m.source_id.to_string() {
                        continue;
                    }
                }
            }
            let chapters = chapter_repo.get_by_manga_id(m.id).await?;
            nodes.push(MangaType::from_model(m, chapters));
        }

        let count = nodes.len() as i32;
        Ok(MangaNodeList {
            nodes,
            page_info: PageInfo::default(),
            total_count: count,
        })
    }

    #[graphql(name = "manga")]
    async fn manga(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<Option<MangaType>> {
        let pool = ctx.data::<SqlitePool>()?;
        let manga_repo = MangaRepository::new(pool);
        let chapter_repo = ChapterRepository::new(pool);

        if let Some(m) = manga_repo.get_by_id(id).await? {
            let chapters = chapter_repo.get_by_manga_id(m.id).await?;
            Ok(Some(MangaType::from_model(m, chapters)))
        } else {
            Ok(None)
        }
    }

    #[graphql(name = "chapters")]
    async fn chapters(
        &self,
        ctx: &Context<'_>,
        condition: Option<ChapterConditionInput>,
        _filter: Option<ChapterFilterInput>,
        _order: Option<Vec<ChapterOrderInput>>,
        _before: Option<Cursor>,
        _after: Option<Cursor>,
        _first: Option<i32>,
        _last: Option<i32>,
        _offset: Option<i32>,
    ) -> async_graphql::Result<ChapterNodeList> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = ChapterRepository::new(pool);
        let manga_repo = MangaRepository::new(pool);

        let chapters = if let Some(cond) = condition {
            if let Some(manga_id) = cond.manga_id {
                repo.get_by_manga_id(manga_id).await?
            } else if let Some(id) = cond.id {
                repo.get_by_id(id).await?.into_iter().collect()
            } else {
                repo.get_all().await?
            }
        } else {
            repo.get_all().await?
        };

        let mut nodes: Vec<ChapterType> = Vec::new();
        for c in chapters {
            let mut node = ChapterType::from(c);
            if let Ok(Some(manga)) = manga_repo.get_by_id(node.manga_id).await {
                let manga_chapters = repo.get_by_manga_id(manga.id).await.unwrap_or_default();
                node.manga = Some(Box::new(MangaType::from_model(manga, manga_chapters)));
            }
            nodes.push(node);
        }

        let count = nodes.len() as i32;

        Ok(ChapterNodeList {
            nodes,
            page_info: PageInfo::default(),
            total_count: count,
        })
    }

    #[graphql(name = "chapter")]
    async fn chapter(
        &self,
        ctx: &Context<'_>,
        id: i64,
    ) -> async_graphql::Result<Option<ChapterType>> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = ChapterRepository::new(pool);
        let c = repo.get_by_id(id).await?;
        Ok(c.map(Into::into))
    }

    #[graphql(name = "sources")]
    async fn sources(
        &self,
        ctx: &Context<'_>,
        _condition: Option<SourceConditionInput>,
        _filter: Option<SourceFilterInput>,
        _order: Option<Vec<SourceOrderInput>>,
        _before: Option<Cursor>,
        _after: Option<Cursor>,
        _first: Option<i32>,
        _last: Option<i32>,
        _offset: Option<i32>,
    ) -> async_graphql::Result<SourceNodeList> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let installed = registry.get_installed_extensions().await;

        let mut nodes = Vec::new();
        for ext in installed {
            if ext.pkg_name == "eu.kanade.tachiyomi.extension.en.allanime"
                || ext.pkg_name == "eu.kanade.tachiyomi.extension.all.mangadex"
                || registry.get_source_by_pkg(&ext.pkg_name).is_some()
            {
                let id = if ext.pkg_name == "eu.kanade.tachiyomi.extension.en.allanime" {
                    "8861274191478178487".to_string()
                } else if ext.pkg_name == "eu.kanade.tachiyomi.extension.all.mangadex" {
                    "2499283573021221994".to_string()
                } else {
                    "1".to_string()
                };

                let ext_type = ExtensionType {
                    pkg_name: ext.pkg_name.clone(),
                    name: ext.name.clone(),
                    lang: ext.lang.clone(),
                    version_code_long: ext.version_code.into(),
                    version_code: ext.version_code as i32,
                    version_name: ext.version_name.clone(),
                    icon_url: ext.icon_url.clone(),
                    apk_url: Some(ext.apk_url.clone()),
                    apk_name: None,
                    jar_url: None,
                    repo: None,
                    store_index_url: Some("https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json".to_string()),
                    content_warning: if ext.is_nsfw {
                        ContentWarning::Nsfw
                    } else {
                        ContentWarning::Safe
                    },
                    is_installed: true,
                    is_nsfw: ext.is_nsfw,
                    is_obsolete: false,
                    has_update: false,
                    extension_lib: None,
                    extension_store: None,
                    source: SourceNodeList::default(),
                };

                nodes.push(SourceType {
                    id: id.into(),
                    name: ext.name.clone(),
                    lang: ext.lang.clone(),
                    display_name: format!("{} ({})", ext.name, ext.lang),
                    icon_url: ext.icon_url.clone(),
                    content_warning: if ext.is_nsfw {
                        ContentWarning::Nsfw
                    } else {
                        ContentWarning::Safe
                    },
                    is_configurable: false,
                    is_nsfw: ext.is_nsfw,
                    supports_latest: true,
                    base_url: None,
                    home_url: None,
                    extension: Some(Box::new(ext_type)),
                    meta: vec![],
                });
            }
        }

        let count = nodes.len() as i32;
        Ok(SourceNodeList {
            nodes,
            page_info: PageInfo::default(),
            total_count: count,
        })
    }

    #[graphql(name = "source")]
    async fn source(
        &self,
        ctx: &Context<'_>,
        id: LongString,
    ) -> async_graphql::Result<Option<SourceType>> {
        let sources = self
            .sources(ctx, None, None, None, None, None, None, None, None)
            .await?;
        Ok(sources.nodes.into_iter().find(|s| s.id.0 == id.0))
    }

    #[graphql(name = "extensions")]
    async fn extensions(
        &self,
        ctx: &Context<'_>,
        _condition: Option<ExtensionConditionInput>,
        _filter: Option<ExtensionFilterInput>,
        _order: Option<Vec<ExtensionOrderInput>>,
        _before: Option<Cursor>,
        _after: Option<Cursor>,
        _first: Option<i32>,
        _last: Option<i32>,
        _offset: Option<i32>,
    ) -> async_graphql::Result<ExtensionNodeList> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let installed = registry.get_installed_extensions().await;
        let available = registry
            .get_available_extensions()
            .await
            .unwrap_or_default();

        let mut nodes = Vec::new();
        let installed_pkgs: std::collections::HashSet<String> =
            installed.iter().map(|e| e.pkg_name.clone()).collect();

        for ext in installed {
            nodes.push(ExtensionType {
                pkg_name: ext.pkg_name.clone(),
                name: ext.name.clone(),
                lang: ext.lang.clone(),
                version_code_long: ext.version_code.into(),
                version_code: ext.version_code as i32,
                version_name: ext.version_name.clone(),
                icon_url: ext.icon_url.clone(),
                apk_url: Some(ext.apk_url.clone()),
                apk_name: None,
                jar_url: None,
                repo: None,
                store_index_url: Some(
                    "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
                        .to_string(),
                ),
                content_warning: if ext.is_nsfw {
                    ContentWarning::Nsfw
                } else {
                    ContentWarning::Safe
                },
                is_installed: true,
                is_nsfw: ext.is_nsfw,
                is_obsolete: false,
                has_update: false,
                extension_lib: None,
                extension_store: None,
                source: SourceNodeList::default(),
            });
        }

        for ext in available {
            if !installed_pkgs.contains(&ext.pkg_name) {
                nodes.push(ExtensionType {
                    pkg_name: ext.pkg_name.clone(),
                    name: ext.name.clone(),
                    lang: ext.lang.clone(),
                    version_code_long: ext.version_code.into(),
                    version_code: ext.version_code as i32,
                    version_name: ext.version_name.clone(),
                    icon_url: ext.icon_url.clone(),
                    apk_url: Some(ext.apk_url.clone()),
                    apk_name: None,
                    jar_url: None,
                    repo: None,
                    store_index_url: Some("https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json".to_string()),
                    content_warning: if ext.is_nsfw {
                        ContentWarning::Nsfw
                    } else {
                        ContentWarning::Safe
                    },
                    is_installed: false,
                    is_nsfw: ext.is_nsfw,
                    is_obsolete: false,
                    has_update: false,
                    extension_lib: None,
                    extension_store: None,
                    source: SourceNodeList::default(),
                });
            }
        }

        let count = nodes.len() as i32;
        Ok(ExtensionNodeList {
            nodes,
            page_info: PageInfo::default(),
            total_count: count,
        })
    }

    #[graphql(name = "extension")]
    async fn extension(
        &self,
        ctx: &Context<'_>,
        pkg_name: String,
    ) -> async_graphql::Result<Option<ExtensionType>> {
        let exts = self
            .extensions(ctx, None, None, None, None, None, None, None, None)
            .await?;
        Ok(exts.nodes.into_iter().find(|e| e.pkg_name == pkg_name))
    }

    #[graphql(name = "extensionStore")]
    async fn extension_store(
        &self,
        index_url: String,
    ) -> async_graphql::Result<Option<ExtensionStoreType>> {
        Ok(Some(ExtensionStoreType {
            badge_label: "Official".to_string(),
            contact_discord: Some("https://discord.gg/keiyoushi".to_string()),
            contact_website: "https://keiyoushi.github.io".to_string(),
            extension_list_url: Some(
                "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
                    .to_string(),
            ),
            index_url,
            is_legacy: false,
            name: "Keiyoushi".to_string(),
            signing_key: "".to_string(),
            extensions: ExtensionNodeList::default(),
        }))
    }

    #[graphql(name = "extensionStores")]
    async fn extension_stores(
        &self,
        _condition: Option<ExtensionStoreConditionInput>,
        _filter: Option<ExtensionStoreFilterInput>,
        _order: Option<Vec<ExtensionStoreOrderInput>>,
        _before: Option<Cursor>,
        _after: Option<Cursor>,
        _first: Option<i32>,
        _last: Option<i32>,
        _offset: Option<i32>,
    ) -> async_graphql::Result<ExtensionStoreNodeList> {
        let store = ExtensionStoreType {
            badge_label: "Official".to_string(),
            contact_discord: Some("https://discord.gg/keiyoushi".to_string()),
            contact_website: "https://keiyoushi.github.io".to_string(),
            extension_list_url: Some(
                "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
                    .to_string(),
            ),
            index_url: "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
                .to_string(),
            is_legacy: false,
            name: "Keiyoushi".to_string(),
            signing_key: "".to_string(),
            extensions: ExtensionNodeList::default(),
        };
        Ok(ExtensionStoreNodeList {
            nodes: vec![store],
            page_info: PageInfo::default(),
            total_count: 1,
        })
    }

    #[graphql(name = "settings")]
    async fn settings(&self) -> async_graphql::Result<SettingsType> {
        Ok(SettingsType {
            ip: "0.0.0.0".to_string(),
            port: 4567,
            socks_proxy_enabled: false,
            socks_proxy_version: 5,
            socks_proxy_host: "".to_string(),
            socks_proxy_port: 1080,
            socks_proxy_username: "".to_string(),
            socks_proxy_password: "".to_string(),
            web_ui_flavor: "WebUI".to_string(),
            initial_open_in_browser_enabled: false,
            web_ui_interface: "browser".to_string(),
            electron_path: "".to_string(),
            web_ui_channel: "stable".to_string(),
            web_ui_update_check_interval: 0,
            download_as_cbz: true,
            downloads_path: "/data/downloads".to_string(),
            auto_download_new_chapters: false,
            exclude_entry_with_unread_chapters: false,
            auto_download_new_chapters_limit: 0,
            auto_download_ignore_re_uploads: false,
            download_conversions: vec![],
            serve_conversions: vec![],
            max_sources_in_parallel: 8,
            exclude_unread_chapters: false,
            exclude_not_started: false,
            exclude_completed: false,
            global_update_interval: 0,
            update_mangas: false,
            auth_mode: "NONE".to_string(),
            auth_password: "".to_string(),
            auth_username: "".to_string(),
            jwt_audience: "".to_string(),
            jwt_token_expiry: 0,
            jwt_refresh_expiry: 0,
            debug_logs_enabled: false,
            system_tray_enabled: false,
            max_log_file_size: 10485760,
            max_log_files: 5,
            max_log_folder_size: 52428800,
            backup_path: "/data/backup".to_string(),
            backup_time: "00:00".to_string(),
            backup_interval: 0,
            backup_ttl: 0,
            auto_backup_include_categories: true,
            auto_backup_include_chapters: true,
            auto_backup_include_client_data: true,
            auto_backup_include_history: true,
            auto_backup_include_manga: true,
            auto_backup_include_server_settings: true,
            auto_backup_include_tracking: true,
            local_source_path: "/data/local".to_string(),
            flare_solverr_enabled: false,
            flare_solverr_url: "".to_string(),
            flare_solverr_timeout: 60,
            flare_solverr_session_name: "".to_string(),
            flare_solverr_session_ttl: 15,
            flare_solverr_as_response_fallback: false,
            opds_use_binary_file_sizes: false,
            opds_items_per_page: 20,
            opds_enable_page_read_progress: false,
            opds_mark_as_read_on_download: false,
            opds_show_only_unread_chapters: false,
            opds_show_only_downloaded_chapters: false,
            opds_chapter_sort_order: "ASC".to_string(),
            opds_cbz_mimetype: "application/zip".to_string(),
            opds_skip_chapter_metadata_feed: false,
            koreader_sync_checksum_method: "".to_string(),
            koreader_sync_strategy_backward: "".to_string(),
            koreader_sync_strategy_forward: "".to_string(),
            koreader_sync_percentage_tolerance: 0,
            database_type: "SQLITE".to_string(),
            database_url: "".to_string(),
            database_username: "".to_string(),
            database_password: "".to_string(),
            use_hikari_connection_pool: false,
            kcef_enabled: false,
            sync_data_categories: false,
            sync_data_chapters: false,
            sync_data_history: false,
            sync_data_manga: false,
            sync_data_tracking: false,
            sync_interval: 0,
            sync_yomi_api_key: "".to_string(),
            sync_yomi_enabled: false,
            sync_yomi_host: "".to_string(),
        })
    }

    #[graphql(name = "downloadStatus")]
    async fn download_status(&self, _ctx: &Context<'_>) -> async_graphql::Result<DownloadStatus> {
        Ok(DownloadStatus {
            state: DownloaderState::Stopped,
            queue: vec![],
        })
    }

    #[graphql(name = "updateStatus")]
    async fn update_status(&self) -> async_graphql::Result<UpdateStatus> {
        Ok(UpdateStatus {
            complete_jobs: 0,
            failed_jobs: 0,
            is_running: false,
            pending_jobs: 0,
            running_jobs: 0,
            skipped_categories: 0,
            skipped_jobs: 0,
            updating_categories: vec![],
        })
    }

    #[graphql(name = "libraryUpdateStatus")]
    async fn library_update_status(&self) -> async_graphql::Result<LibraryUpdateStatus> {
        Ok(LibraryUpdateStatus {
            category_updates: vec![],
            jobs_info: "idle".to_string(),
            manga_updates: vec![],
        })
    }

    #[graphql(name = "lastUpdateTimestamp")]
    async fn last_update_timestamp(&self) -> async_graphql::Result<LastUpdateTimestampPayload> {
        Ok(LastUpdateTimestampPayload {
            timestamp: "0".into(),
        })
    }

    #[graphql(name = "checkForWebUIUpdate")]
    async fn check_for_web_ui_update(&self) -> async_graphql::Result<WebUIUpdateCheck> {
        Ok(WebUIUpdateCheck {
            channel: "stable".to_string(),
            tag: "latest".to_string(),
            update_available: false,
        })
    }

    #[graphql(name = "getWebUIUpdateStatus")]
    async fn get_web_ui_update_status(&self) -> async_graphql::Result<WebUIUpdateStatus> {
        Ok(WebUIUpdateStatus {
            info: "up-to-date".to_string(),
            progress: 1.0,
            state: "COMPLETED".to_string(),
        })
    }

    #[graphql(name = "checkForServerUpdates")]
    async fn check_for_server_updates(
        &self,
    ) -> async_graphql::Result<Vec<CheckForServerUpdatesPayload>> {
        Ok(vec![])
    }

    #[graphql(name = "metas")]
    async fn metas(&self) -> async_graphql::Result<GlobalMetaNodeList> {
        Ok(GlobalMetaNodeList::default())
    }

    // --- Legacy / Direct Helper Queries ---

    #[graphql(name = "availableExtensions")]
    async fn available_extensions(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<ExtensionListing>> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let result = registry
            .get_available_extensions()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(result.into_iter().map(|ext| ext.into()).collect())
    }

    #[graphql(name = "installedExtensions")]
    async fn installed_extensions(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<ExtensionListing>> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let installed = registry.get_installed_extensions().await;
        Ok(installed.into_iter().map(|ext| ext.into()).collect())
    }

    #[graphql(name = "extensionRepos")]
    async fn extension_repos(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Vec<ExtensionRepo>> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        Ok(registry.repos.iter().cloned().map(|r| r.into()).collect())
    }

    #[graphql(name = "sourceManga")]
    async fn source_manga(
        &self,
        ctx: &Context<'_>,
        source_id: String,
        type_name: String,
        page: i32,
        query: Option<String>,
    ) -> async_graphql::Result<MangaPagePayload> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let sid = source_id
            .parse::<i64>()
            .map_err(|_| async_graphql::Error::new("Invalid source ID"))?;

        let source = registry
            .get_source(sid)
            .ok_or_else(|| async_graphql::Error::new(format!("Source {} not found", sid)))?;

        let page_res = if query.is_some() || type_name.to_uppercase() == "SEARCH" {
            source
                .search_manga(query.as_deref().unwrap_or(""), &[], page)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        } else if type_name.to_uppercase() == "LATEST" {
            source
                .get_latest_updates(page)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        } else {
            source
                .get_popular_manga(page)
                .await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        };

        let manga_types: Vec<MangaType> = page_res
            .manga_list
            .into_iter()
            .map(|m| MangaType::from_model(m, vec![]))
            .collect();

        Ok(MangaPagePayload {
            mangas: manga_types,
            has_next_page: page_res.has_next_page,
        })
    }

    #[graphql(name = "chapterPages")]
    async fn chapter_pages(
        &self,
        ctx: &Context<'_>,
        chapter_id: i64,
    ) -> async_graphql::Result<Vec<Page>> {
        let pool = ctx.data::<SqlitePool>()?;
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;

        let chapter = ChapterRepository::new(pool)
            .get_by_id(chapter_id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new(format!("Chapter {} not found", chapter_id))
            })?;

        let manga = MangaRepository::new(pool)
            .get_by_id(chapter.manga_id)
            .await?
            .ok_or_else(|| {
                async_graphql::Error::new(format!("Manga {} not found", chapter.manga_id))
            })?;

        let source = registry.get_source(manga.source_id).ok_or_else(|| {
            async_graphql::Error::new(format!("Source {} not found", manga.source_id))
        })?;

        let chapter_model: suwayomi_core::models::Chapter = chapter;
        let pages = source
            .get_page_list(&chapter_model)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(pages
            .into_iter()
            .map(|p| Page {
                index: p.index,
                url: p.url,
                image_url: p.image_url,
            })
            .collect())
    }
}

// --- Mutation Root ---

#[derive(InputObject, Default, Clone)]
pub struct FetchMangaAndChaptersInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "fetchChapters")]
    pub fetch_chapters: Option<bool>,
    #[graphql(name = "fetchManga")]
    pub fetch_manga: Option<bool>,
    pub id: i64,
}

#[derive(SimpleObject, Clone)]
pub struct FetchMangaAndChaptersPayload {
    pub manga: Option<MangaType>,
    pub chapters: Option<Vec<ChapterType>>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct FetchChapterPagesInput {
    #[graphql(name = "chapterId")]
    pub chapter_id: i64,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub format: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct FetchChapterPagesPayload {
    pub chapter: Option<ChapterType>,
    pub pages: Option<Vec<Page>>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct UpdateMangaPatchInput {
    #[graphql(name = "inLibrary")]
    pub in_library: Option<bool>,
}

#[derive(InputObject, Default, Clone)]
pub struct UpdateMangaInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub id: i64,
    pub patch: Option<UpdateMangaPatchInput>,
}

#[derive(SimpleObject, Clone)]
pub struct UpdateMangaPayload {
    pub manga: Option<MangaType>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct UpdateLibraryMangaInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct UpdateLibraryMangaPayload {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "updateStatus")]
    pub update_status: Option<UpdateStatus>,
}

#[derive(InputObject, Default, Clone)]
pub struct UpdateChapterPatchInput {
    #[graphql(name = "isRead")]
    pub is_read: Option<bool>,
    #[graphql(name = "isBookmarked")]
    pub is_bookmarked: Option<bool>,
    #[graphql(name = "lastPageRead")]
    pub last_page_read: Option<i64>,
}

#[derive(InputObject, Default, Clone)]
pub struct UpdateChapterInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub id: i64,
    pub patch: Option<UpdateChapterPatchInput>,
}

#[derive(SimpleObject, Clone)]
pub struct UpdateChapterPayload {
    pub chapter: Option<ChapterType>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct UpdateChaptersInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub ids: Vec<i64>,
    pub patch: Option<UpdateChapterPatchInput>,
}

#[derive(SimpleObject, Clone)]
pub struct UpdateChaptersPayload {
    pub chapters: Option<Vec<ChapterType>>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct CreateCategoryInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub name: String,
    pub order: Option<i32>,
    pub default: Option<bool>,
}

#[derive(SimpleObject, Clone)]
pub struct CreateCategoryPayload {
    pub category: Option<CategoryType>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct DeleteCategoryInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "categoryId")]
    pub category_id: i64,
}

#[derive(SimpleObject, Clone)]
pub struct DeleteCategoryPayload {
    pub category: Option<CategoryType>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct EnqueueChapterDownloadInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub id: i64,
}

#[derive(SimpleObject, Clone)]
pub struct EnqueueChapterDownloadPayload {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "downloadStatus")]
    pub download_status: Option<DownloadStatus>,
}

#[derive(InputObject, Default, Clone)]
pub struct StartDownloaderInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct StartDownloaderPayload {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "downloadStatus")]
    pub download_status: Option<DownloadStatus>,
}

#[derive(InputObject, Default, Clone)]
pub struct StopDownloaderInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct StopDownloaderPayload {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "downloadStatus")]
    pub download_status: Option<DownloadStatus>,
}

#[derive(InputObject, Default, Clone)]
pub struct ClearDownloaderInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct ClearDownloaderPayload {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "downloadStatus")]
    pub download_status: Option<DownloadStatus>,
}

#[derive(InputObject, Default, Clone)]
pub struct InstallExternalExtensionInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub file: Option<String>,
}

#[derive(SimpleObject, Clone)]
pub struct InstallExternalExtensionPayload {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub extension: Option<ExtensionType>,
}

#[derive(InputObject, Default, Clone)]
pub struct FetchExtensionsInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
}

#[derive(InputObject, Default, Clone)]
pub struct MetaInput {
    pub key: String,
    pub value: String,
}

#[derive(InputObject, Default, Clone)]
pub struct SetCategoryMetasItemInput {
    #[graphql(name = "categoryIds")]
    pub category_ids: Vec<i64>,
    pub metas: Vec<MetaInput>,
}

#[derive(InputObject, Default, Clone)]
pub struct SetCategoryMetasInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub items: Vec<SetCategoryMetasItemInput>,
}

#[derive(SimpleObject, Default, Clone)]
pub struct SetCategoryMetasPayload {
    pub categories: Option<Vec<CategoryType>>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub metas: Option<Vec<CategoryMetaType>>,
}

#[derive(InputObject, Default, Clone)]
pub struct DeleteCategoryMetasItemInput {
    #[graphql(name = "categoryIds")]
    pub category_ids: Vec<i64>,
    pub keys: Option<Vec<String>>,
    pub prefixes: Option<Vec<String>>,
}

#[derive(InputObject, Default, Clone)]
pub struct DeleteCategoryMetasInput {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub items: Vec<DeleteCategoryMetasItemInput>,
}

#[derive(SimpleObject, Default, Clone)]
pub struct DeleteCategoryMetasPayload {
    pub categories: Option<Vec<CategoryType>>,
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    pub metas: Option<Vec<CategoryMetaType>>,
}

#[derive(SimpleObject, Clone)]
pub struct FetchExtensionsPayload {
    #[graphql(name = "clientMutationId")]
    pub client_mutation_id: Option<String>,
    #[graphql(name = "extensionStores")]
    pub extension_stores: Option<Vec<ExtensionStoreType>>,
    pub extensions: Option<Vec<ExtensionType>>,
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    #[graphql(name = "fetchMangaAndChapters")]
    async fn fetch_manga_and_chapters_mutation(
        &self,
        ctx: &Context<'_>,
        input: FetchMangaAndChaptersInput,
    ) -> async_graphql::Result<FetchMangaAndChaptersPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let manga_repo = MangaRepository::new(pool);
        let chapter_repo = ChapterRepository::new(pool);
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;

        let manga = manga_repo.get_by_id(input.id).await?;
        if let Some(mut m) = manga {
            if let Some(source) = registry.get_source(m.source_id) {
                if input.fetch_manga.unwrap_or(true) {
                    if let Ok(updated) = source.get_manga_details(m.clone()).await {
                        let _ = manga_repo.update(&updated).await;
                        m = updated;
                    }
                }
                if input.fetch_chapters.unwrap_or(true) {
                    if let Ok(mut chapters) = source.get_chapter_list(&m).await {
                        for c in &mut chapters {
                            c.manga_id = m.id;
                        }
                        let _ = chapter_repo.insert_chapters(&chapters).await;
                    }
                }
            }
            let chapters = chapter_repo.get_by_manga_id(m.id).await?;
            let manga_type = MangaType::from_model(m, chapters.clone());
            let chapter_types = chapters.into_iter().map(Into::into).collect();
            Ok(FetchMangaAndChaptersPayload {
                manga: Some(manga_type),
                chapters: Some(chapter_types),
                client_mutation_id: input.client_mutation_id,
            })
        } else {
            Err(async_graphql::Error::new(format!(
                "Manga {} not found",
                input.id
            )))
        }
    }

    #[graphql(name = "fetchChapterPages")]
    async fn fetch_chapter_pages_mutation(
        &self,
        ctx: &Context<'_>,
        input: FetchChapterPagesInput,
    ) -> async_graphql::Result<FetchChapterPagesPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let chapter_repo = ChapterRepository::new(pool);
        let manga_repo = MangaRepository::new(pool);
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;

        let chapter = chapter_repo.get_by_id(input.chapter_id).await?;
        if let Some(c) = chapter {
            let manga = manga_repo.get_by_id(c.manga_id).await?;
            let mut page_list = vec![];
            if let Some(m) = manga {
                if let Some(source) = registry.get_source(m.source_id) {
                    if let Ok(pages) = source.get_page_list(&c).await {
                        page_list = pages
                            .into_iter()
                            .map(|p| Page {
                                index: p.index,
                                url: p.url,
                                image_url: p.image_url,
                            })
                            .collect();
                    }
                }
            }
            Ok(FetchChapterPagesPayload {
                chapter: Some(c.into()),
                pages: Some(page_list),
                client_mutation_id: input.client_mutation_id,
            })
        } else {
            Ok(FetchChapterPagesPayload {
                chapter: None,
                pages: None,
                client_mutation_id: input.client_mutation_id,
            })
        }
    }

    #[graphql(name = "updateManga")]
    async fn update_manga_mutation(
        &self,
        ctx: &Context<'_>,
        input: UpdateMangaInput,
    ) -> async_graphql::Result<UpdateMangaPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = MangaRepository::new(pool);
        let chapter_repo = ChapterRepository::new(pool);

        if let Some(patch) = input.patch {
            if let Some(in_library) = patch.in_library {
                repo.update_in_library(input.id, in_library).await?;
            }
        }
        let manga = repo.get_by_id(input.id).await?;
        let manga_type = if let Some(m) = manga {
            let chapters = chapter_repo.get_by_manga_id(m.id).await?;
            Some(MangaType::from_model(m, chapters))
        } else {
            None
        };
        Ok(UpdateMangaPayload {
            manga: manga_type,
            client_mutation_id: input.client_mutation_id,
        })
    }

    #[graphql(name = "updateLibraryManga")]
    async fn update_library_manga_mutation(
        &self,
        input: UpdateLibraryMangaInput,
    ) -> async_graphql::Result<UpdateLibraryMangaPayload> {
        Ok(UpdateLibraryMangaPayload {
            client_mutation_id: input.client_mutation_id,
            update_status: Some(UpdateStatus {
                complete_jobs: 0,
                failed_jobs: 0,
                is_running: false,
                pending_jobs: 0,
                running_jobs: 0,
                skipped_categories: 0,
                skipped_jobs: 0,
                updating_categories: vec![],
            }),
        })
    }

    #[graphql(name = "updateChapter")]
    async fn update_chapter_mutation(
        &self,
        ctx: &Context<'_>,
        input: UpdateChapterInput,
    ) -> async_graphql::Result<UpdateChapterPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = ChapterRepository::new(pool);

        if let Some(patch) = input.patch {
            if let Some(read) = patch.is_read {
                let last_page = patch.last_page_read.unwrap_or(0);
                repo.update_read(input.id, read, last_page).await?;
            }
        }
        let chapter = repo.get_by_id(input.id).await?;
        Ok(UpdateChapterPayload {
            chapter: chapter.map(Into::into),
            client_mutation_id: input.client_mutation_id,
        })
    }

    #[graphql(name = "updateChapters")]
    async fn update_chapters_mutation(
        &self,
        ctx: &Context<'_>,
        input: UpdateChaptersInput,
    ) -> async_graphql::Result<UpdateChaptersPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = ChapterRepository::new(pool);

        let mut updated = Vec::new();
        if let Some(patch) = input.patch {
            if let Some(read) = patch.is_read {
                let last_page = patch.last_page_read.unwrap_or(0);
                for id in &input.ids {
                    let _ = repo.update_read(*id, read, last_page).await;
                    if let Ok(Some(c)) = repo.get_by_id(*id).await {
                        updated.push(c.into());
                    }
                }
            }
        }
        Ok(UpdateChaptersPayload {
            chapters: Some(updated),
            client_mutation_id: input.client_mutation_id,
        })
    }

    #[graphql(name = "createCategory")]
    async fn create_category_mutation(
        &self,
        ctx: &Context<'_>,
        input: CreateCategoryInput,
    ) -> async_graphql::Result<CreateCategoryPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = CategoryRepository::new(pool);
        let category = models::Category {
            id: 0,
            name: input.name,
            order: input.order.unwrap_or(0),
            flags: 0,
        };
        let id = repo.insert_category(&category).await?;
        let cat = repo.get_by_id(id).await?;
        Ok(CreateCategoryPayload {
            category: cat.map(|c| CategoryType::from_model(c, 0)),
            client_mutation_id: input.client_mutation_id,
        })
    }

    #[graphql(name = "deleteCategory")]
    async fn delete_category_mutation(
        &self,
        ctx: &Context<'_>,
        input: DeleteCategoryInput,
    ) -> async_graphql::Result<DeleteCategoryPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = CategoryRepository::new(pool);
        let cat = repo.get_by_id(input.category_id).await?;
        repo.delete_category(input.category_id).await?;
        Ok(DeleteCategoryPayload {
            category: cat.map(|c| CategoryType::from_model(c, 0)),
            client_mutation_id: input.client_mutation_id,
        })
    }

    #[graphql(name = "enqueueChapterDownload")]
    async fn enqueue_chapter_download_mutation(
        &self,
        ctx: &Context<'_>,
        input: EnqueueChapterDownloadInput,
    ) -> async_graphql::Result<EnqueueChapterDownloadPayload> {
        let pool = ctx.data::<SqlitePool>()?;
        let chapter_repo = ChapterRepository::new(pool);

        if let Ok(Some(chapter)) = chapter_repo.get_by_id(input.id).await {
            let queue = ctx.data::<suwayomi_downloader::queue::DownloadQueue>()?;
            let item = suwayomi_core::models::DownloadQueueItem {
                chapter_id: chapter.id,
                manga_id: chapter.manga_id,
                status: suwayomi_core::models::DownloadStatus::Queued,
            };
            queue.enqueue(item).await;
        }

        Ok(EnqueueChapterDownloadPayload {
            client_mutation_id: input.client_mutation_id,
            download_status: Some(DownloadStatus {
                state: DownloaderState::Started,
                queue: vec![],
            }),
        })
    }

    #[graphql(name = "startDownloader")]
    async fn start_downloader_mutation(
        &self,
        ctx: &Context<'_>,
        input: StartDownloaderInput,
    ) -> async_graphql::Result<StartDownloaderPayload> {
        if let Ok(queue) = ctx.data::<suwayomi_downloader::queue::DownloadQueue>() {
            queue.resume().await;
        }
        Ok(StartDownloaderPayload {
            client_mutation_id: input.client_mutation_id,
            download_status: Some(DownloadStatus {
                state: DownloaderState::Started,
                queue: vec![],
            }),
        })
    }

    #[graphql(name = "stopDownloader")]
    async fn stop_downloader_mutation(
        &self,
        ctx: &Context<'_>,
        input: StopDownloaderInput,
    ) -> async_graphql::Result<StopDownloaderPayload> {
        if let Ok(queue) = ctx.data::<suwayomi_downloader::queue::DownloadQueue>() {
            queue.pause().await;
        }
        Ok(StopDownloaderPayload {
            client_mutation_id: input.client_mutation_id,
            download_status: Some(DownloadStatus {
                state: DownloaderState::Stopped,
                queue: vec![],
            }),
        })
    }

    #[graphql(name = "clearDownloader")]
    async fn clear_downloader_mutation(
        &self,
        input: ClearDownloaderInput,
    ) -> async_graphql::Result<ClearDownloaderPayload> {
        Ok(ClearDownloaderPayload {
            client_mutation_id: input.client_mutation_id,
            download_status: Some(DownloadStatus {
                state: DownloaderState::Stopped,
                queue: vec![],
            }),
        })
    }

    #[graphql(name = "installExternalExtension")]
    async fn install_external_extension_mutation(
        &self,
        input: InstallExternalExtensionInput,
    ) -> async_graphql::Result<InstallExternalExtensionPayload> {
        Ok(InstallExternalExtensionPayload {
            client_mutation_id: input.client_mutation_id,
            extension: None,
        })
    }

    #[graphql(name = "setCategoryMetas")]
    async fn set_category_metas(
        &self,
        input: SetCategoryMetasInput,
    ) -> async_graphql::Result<SetCategoryMetasPayload> {
        let mut metas = Vec::new();
        for item in &input.items {
            for cid in &item.category_ids {
                for m in &item.metas {
                    metas.push(CategoryMetaType {
                        category_id: *cid,
                        key: m.key.clone(),
                        value: m.value.clone(),
                    });
                }
            }
        }
        Ok(SetCategoryMetasPayload {
            categories: None,
            client_mutation_id: input.client_mutation_id,
            metas: Some(metas),
        })
    }

    #[graphql(name = "deleteCategoryMetas")]
    async fn delete_category_metas(
        &self,
        input: DeleteCategoryMetasInput,
    ) -> async_graphql::Result<DeleteCategoryMetasPayload> {
        Ok(DeleteCategoryMetasPayload {
            categories: None,
            client_mutation_id: input.client_mutation_id,
            metas: Some(vec![]),
        })
    }

    #[graphql(name = "fetchExtensions")]
    async fn fetch_extensions_mutation(
        &self,
        ctx: &Context<'_>,
        input: FetchExtensionsInput,
    ) -> async_graphql::Result<FetchExtensionsPayload> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let _ = registry.get_available_extensions().await;
        let exts = registry.get_installed_extensions().await;
        let mut nodes = Vec::new();
        for ext in exts {
            nodes.push(ExtensionType {
                pkg_name: ext.pkg_name.clone(),
                name: ext.name.clone(),
                lang: ext.lang.clone(),
                version_code_long: ext.version_code.into(),
                version_code: ext.version_code as i32,
                version_name: ext.version_name.clone(),
                icon_url: ext.icon_url.clone(),
                apk_url: Some(ext.apk_url.clone()),
                apk_name: None,
                jar_url: None,
                repo: None,
                store_index_url: Some(
                    "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
                        .to_string(),
                ),
                content_warning: if ext.is_nsfw {
                    ContentWarning::Nsfw
                } else {
                    ContentWarning::Safe
                },
                is_installed: true,
                is_nsfw: ext.is_nsfw,
                is_obsolete: false,
                has_update: false,
                extension_lib: None,
                extension_store: None,
                source: SourceNodeList::default(),
            });
        }
        let store = ExtensionStoreType {
            badge_label: "Official".to_string(),
            contact_discord: Some("https://discord.gg/keiyoushi".to_string()),
            contact_website: "https://keiyoushi.github.io".to_string(),
            extension_list_url: Some(
                "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
                    .to_string(),
            ),
            index_url: "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.min.json"
                .to_string(),
            is_legacy: false,
            name: "Keiyoushi".to_string(),
            signing_key: "".to_string(),
            extensions: ExtensionNodeList::default(),
        };
        Ok(FetchExtensionsPayload {
            client_mutation_id: input.client_mutation_id,
            extension_stores: Some(vec![store]),
            extensions: Some(nodes),
        })
    }

    // --- Legacy Named Mutations ---

    #[graphql(name = "installExtension")]
    async fn install_extension(
        &self,
        ctx: &Context<'_>,
        pkg_name: String,
    ) -> async_graphql::Result<ExtensionListing> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let listing = registry
            .install_extension(&pkg_name)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(listing.into())
    }

    #[graphql(name = "uninstallExtension")]
    async fn uninstall_extension(
        &self,
        ctx: &Context<'_>,
        pkg_name: String,
    ) -> async_graphql::Result<bool> {
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let result = registry
            .uninstall_extension(&pkg_name)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(result)
    }

    #[graphql(name = "updateMangaInLibrary")]
    async fn update_manga_in_library(
        &self,
        ctx: &Context<'_>,
        id: i64,
        in_library: bool,
    ) -> async_graphql::Result<MangaType> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = MangaRepository::new(pool);
        let chapter_repo = ChapterRepository::new(pool);
        repo.update_in_library(id, in_library).await?;
        let manga = repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| async_graphql::Error::new(format!("Manga {} not found", id)))?;
        let chapters = chapter_repo.get_by_manga_id(id).await?;
        Ok(MangaType::from_model(manga, chapters))
    }

    async fn update_chapter_read(
        &self,
        ctx: &Context<'_>,
        id: i64,
        read: bool,
        last_page_read: i64,
    ) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = ChapterRepository::new(pool);
        repo.update_read(id, read, last_page_read).await?;
        Ok(true)
    }

    async fn create_category(
        &self,
        ctx: &Context<'_>,
        name: String,
        order: i32,
        flags: i32,
    ) -> async_graphql::Result<i64> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = CategoryRepository::new(pool);
        let category = models::Category {
            id: 0,
            name,
            order,
            flags,
        };
        let id = repo.insert_category(&category).await?;
        Ok(id)
    }

    async fn delete_category(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = CategoryRepository::new(pool);
        repo.delete_category(id).await?;
        Ok(true)
    }

    #[graphql(name = "downloadChapter")]
    async fn download_chapter(
        &self,
        ctx: &Context<'_>,
        chapter_id: i64,
    ) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let chapter_repo = ChapterRepository::new(pool);

        let chapter = chapter_repo.get_by_id(chapter_id).await?.ok_or_else(|| {
            async_graphql::Error::new(format!("Chapter {} not found", chapter_id))
        })?;

        let queue = ctx.data::<suwayomi_downloader::queue::DownloadQueue>()?;

        let item = suwayomi_core::models::DownloadQueueItem {
            chapter_id: chapter.id,
            manga_id: chapter.manga_id,
            status: suwayomi_core::models::DownloadStatus::Queued,
        };

        queue.enqueue(item).await;

        Ok(true)
    }

    #[graphql(name = "pauseDownloads")]
    async fn pause_downloads(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let queue = ctx.data::<suwayomi_downloader::queue::DownloadQueue>()?;
        queue.pause().await;
        Ok(true)
    }

    #[graphql(name = "resumeDownloads")]
    async fn resume_downloads(&self, ctx: &Context<'_>) -> async_graphql::Result<bool> {
        let queue = ctx.data::<suwayomi_downloader::queue::DownloadQueue>()?;
        queue.resume().await;
        Ok(true)
    }

    #[graphql(name = "fetchMangaAndChaptersDirect")]
    async fn fetch_manga_and_chapters_direct(
        &self,
        ctx: &Context<'_>,
        source_id: String,
        manga_url: String,
    ) -> async_graphql::Result<MangaType> {
        let pool = ctx.data::<SqlitePool>()?;
        let registry =
            ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;

        let sid = source_id
            .parse::<i64>()
            .map_err(|_| async_graphql::Error::new("Invalid source ID"))?;
        let source = registry
            .get_source(sid)
            .ok_or_else(|| async_graphql::Error::new(format!("Source {} not found", sid)))?;

        let manga_repo = MangaRepository::new(pool);
        let existing_manga = manga_repo.get_by_url(&manga_url).await?;

        let manga = if let Some(m) = existing_manga {
            m
        } else {
            suwayomi_core::models::Manga {
                id: 0,
                source_id: sid,
                url: manga_url.clone(),
                title: "Loading...".to_string(),
                artist: None,
                author: None,
                description: None,
                genre: None,
                status: suwayomi_core::models::MangaStatus::Ongoing,
                thumbnail_url: None,
                update_strategy: 0,
                initialized: false,
            }
        };

        let mut updated_manga = source
            .get_manga_details(manga)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if updated_manga.id == 0 {
            let new_id = manga_repo.insert(&updated_manga).await?;
            updated_manga.id = new_id;
        } else {
            manga_repo.update(&updated_manga).await?;
        }

        let mut chapters = source
            .get_chapter_list(&updated_manga)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        for chapter in &mut chapters {
            chapter.manga_id = updated_manga.id;
        }

        ChapterRepository::new(pool)
            .insert_chapters(&chapters)
            .await?;

        Ok(MangaType::from_model(updated_manga, chapters))
    }
}

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

pub async fn graphql_get_handler(
    State(schema): State<AppSchema>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let is_ws = req
        .headers()
        .get(axum::http::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    if is_ws {
        let (mut parts, _body) = req.into_parts();
        let protocol_res = GraphQLProtocol::from_request_parts(&mut parts, &schema).await;
        let ws_res = WebSocketUpgrade::from_request_parts(&mut parts, &schema).await;

        if let (Ok(protocol), Ok(ws)) = (protocol_res, ws_res) {
            return ws
                .protocols(["graphql-transport-ws", "graphql-ws"])
                .on_upgrade(move |socket| GraphQLWebSocket::new(socket, schema, protocol).serve())
                .into_response();
        }
    }

    graphql_playground().await.into_response()
}

pub async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}
