use async_graphql::{
    Context, EmptySubscription, Enum, Object, Schema, SimpleObject,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use sqlx::SqlitePool;

use suwayomi_core::models;
use suwayomi_db::repositories::{CategoryRepository, ChapterRepository, MangaRepository};

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

impl Into<models::MangaStatus> for MangaStatus {
    fn into(self) -> models::MangaStatus {
        match self {
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

#[derive(SimpleObject, Clone)]
pub struct Manga {
    pub id: i64,
    #[graphql(name = "sourceId")]
    pub source_id: i64,
    pub url: String,
    pub title: String,
    pub artist: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub genre: Option<Vec<String>>,
    pub status: MangaStatus,
    #[graphql(name = "thumbnailUrl")]
    pub thumbnail_url: Option<String>,
    pub update_strategy: i32,
    #[graphql(name = "isInitialized")]
    pub initialized: bool,
    #[graphql(name = "inLibrary")]
    pub in_library: bool,
    #[graphql(name = "unreadCount")]
    pub unread_count: i32,
    #[graphql(name = "downloadCount")]
    pub download_count: i32,
    #[graphql(name = "realUrl")]
    pub real_url: String,
}

impl From<models::Manga> for Manga {
    fn from(manga: models::Manga) -> Self {
        Self {
            id: manga.id,
            source_id: manga.source_id,
            url: manga.url.clone(),
            title: manga.title,
            artist: manga.artist,
            author: manga.author,
            description: manga.description,
            genre: manga.genre,
            status: manga.status.into(),
            thumbnail_url: manga.thumbnail_url,
            update_strategy: manga.update_strategy,
            initialized: manga.initialized,
            in_library: true, // Defaulting to true for library items, could be based on db if available
            unread_count: 0,
            download_count: 0,
            real_url: manga.url, // Defaulting to url
        }
    }
}

impl Into<models::Manga> for Manga {
    fn into(self) -> models::Manga {
        models::Manga {
            id: self.id,
            source_id: self.source_id,
            url: self.url,
            title: self.title,
            artist: self.artist,
            author: self.author,
            description: self.description,
            genre: self.genre,
            status: self.status.into(),
            thumbnail_url: self.thumbnail_url,
            update_strategy: self.update_strategy,
            initialized: self.initialized,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Chapter {
    pub id: i64,
    #[graphql(name = "mangaId")]
    pub manga_id: i64,
    pub url: String,
    pub name: String,
    #[graphql(name = "dateUpload")]
    pub date_upload: i64,
    #[graphql(name = "chapterNumber")]
    pub chapter_number: f32,
    pub scanlator: Option<String>,
    #[graphql(name = "isRead")]
    pub read: bool,
    #[graphql(name = "isBookmarked")]
    pub bookmark: bool,
    #[graphql(name = "lastPageRead")]
    pub last_page_read: i64,
    #[graphql(name = "dateFetch")]
    pub date_fetch: i64,
    #[graphql(name = "sourceOrder")]
    pub source_order: i64,
    #[graphql(name = "pageCount")]
    pub page_count: i32,
    #[graphql(name = "isDownloaded")]
    pub is_downloaded: bool,
}

impl From<models::Chapter> for Chapter {
    fn from(chapter: models::Chapter) -> Self {
        Self {
            id: chapter.id,
            manga_id: chapter.manga_id,
            url: chapter.url,
            name: chapter.name,
            date_upload: chapter.date_upload,
            chapter_number: chapter.chapter_number,
            scanlator: chapter.scanlator,
            read: chapter.read,
            bookmark: chapter.bookmark,
            last_page_read: chapter.last_page_read,
            date_fetch: chapter.date_fetch,
            source_order: chapter.source_order,
            page_count: 0,
            is_downloaded: false,
        }
    }
}

impl Into<models::Chapter> for Chapter {
    fn into(self) -> models::Chapter {
        models::Chapter {
            id: self.id,
            manga_id: self.manga_id,
            url: self.url,
            name: self.name,
            date_upload: self.date_upload,
            chapter_number: self.chapter_number,
            scanlator: self.scanlator,
            read: self.read,
            bookmark: self.bookmark,
            last_page_read: self.last_page_read,
            date_fetch: self.date_fetch,
            source_order: self.source_order,
        }
    }
}

#[derive(SimpleObject, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub order: i32,
    pub flags: i32,
}

impl From<models::Category> for Category {
    fn from(category: models::Category) -> Self {
        Self {
            id: category.id,
            name: category.name,
            order: category.order,
            flags: category.flags,
        }
    }
}

impl Into<models::Category> for Category {
    fn into(self) -> models::Category {
        models::Category {
            id: self.id,
            name: self.name,
            order: self.order,
            flags: self.flags,
        }
    }
}


#[derive(SimpleObject)]
pub struct AboutServerPayload {
    pub name: String,
    pub version: String,
    pub build_time: String,
    pub build_type: String,
    pub discord: String,
    pub github: String,
}

#[derive(SimpleObject)]
pub struct AboutWebUI {
    pub channel: String,
    pub tag: String,
    pub update_timestamp: String,
}

#[derive(SimpleObject)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject)]
pub struct Meta {
    pub key: String,
    pub value: String,
}

#[derive(SimpleObject)]
pub struct DownloadStatusPayload {
    pub queued: i32,
    pub downloading: i32,
    pub downloaded: i32,
    pub error: i32,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    #[graphql(name = "aboutServer")]
    async fn about_server(&self) -> async_graphql::Result<AboutServerPayload> {
        Ok(AboutServerPayload {
            name: "Suwayomi-Rust".to_string(),
            version: "0.1.0".to_string(),
            build_time: "unknown".to_string(),
            build_type: "debug".to_string(),
            discord: "".to_string(),
            github: "https://github.com/Shaik2195/suwayomi-rust".to_string(),
        })
    }

    #[graphql(name = "aboutWebUI")]
    async fn about_web_ui(&self) -> async_graphql::Result<AboutWebUI> {
        Ok(AboutWebUI {
            channel: "stable".to_string(),
            tag: "latest".to_string(),
            update_timestamp: "unknown".to_string(),
        })
    }

    async fn settings(&self) -> async_graphql::Result<Vec<Setting>> {
        Ok(vec![])
    }

    async fn metas(&self) -> async_graphql::Result<Vec<Meta>> {
        Ok(vec![])
    }

    #[graphql(name = "downloadStatus")]
    async fn download_status(&self) -> async_graphql::Result<DownloadStatusPayload> {
        Ok(DownloadStatusPayload {
            queued: 0,
            downloading: 0,
            downloaded: 0,
            error: 0,
        })
    }

    async fn manga(&self, ctx: &Context<'_>, id: i64) -> async_graphql::Result<Option<Manga>> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = MangaRepository::new(pool);
        let result = repo.get_by_id(id).await?;
        Ok(result.map(|m| m.into()))
    }

    async fn library(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Manga>> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = MangaRepository::new(pool);
        let result = repo.get_library().await?;
        Ok(result.into_iter().map(|m| m.into()).collect())
    }

    async fn chapters(&self, ctx: &Context<'_>, manga_id: i64) -> async_graphql::Result<Vec<Chapter>> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = ChapterRepository::new(pool);
        let result = repo.get_by_manga_id(manga_id).await?;
        Ok(result.into_iter().map(|c| c.into()).collect())
    }

    async fn categories(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Category>> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = CategoryRepository::new(pool);
        let result = repo.get_categories().await?;
        Ok(result.into_iter().map(|c| c.into()).collect())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
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
}

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub async fn graphql_handler(
    State(schema): State<AppSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

pub async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}
