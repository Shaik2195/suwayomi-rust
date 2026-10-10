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
            in_library: manga.initialized,
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

#[derive(async_graphql::SimpleObject, Clone)]
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

#[derive(async_graphql::SimpleObject, Clone)]
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

#[derive(async_graphql::SimpleObject, Clone)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub lang: String,
    #[graphql(name = "displayName")]
    pub display_name: String,
    #[graphql(name = "iconUrl")]
    pub icon_url: String,
    #[graphql(name = "supportsLatest")]
    pub supports_latest: bool,
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct Page {
    pub index: i32,
    pub url: String,
    #[graphql(name = "imageUrl")]
    pub image_url: Option<String>,
}

#[derive(async_graphql::SimpleObject, Clone)]
pub struct MangaPagePayload {
    pub mangas: Vec<Manga>,
    #[graphql(name = "hasNextPage")]
    pub has_next_page: bool,
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
    async fn download_status(&self, ctx: &Context<'_>) -> async_graphql::Result<DownloadStatusPayload> {
        let queue = ctx.data::<suwayomi_downloader::queue::DownloadQueue>()?;
        let all_items = queue.get_all().await;
        
        let mut queued = 0;
        let mut downloading = 0;
        let mut downloaded = 0;
        let mut error = 0;

        for item in all_items {
            match item.status {
                suwayomi_core::models::DownloadStatus::Queued => queued += 1,
                suwayomi_core::models::DownloadStatus::Downloading => downloading += 1,
                suwayomi_core::models::DownloadStatus::Downloaded => downloaded += 1,
                suwayomi_core::models::DownloadStatus::Error => error += 1,
            }
        }

        Ok(DownloadStatusPayload {
            queued,
            downloading,
            downloaded,
            error,
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

    async fn mangas(&self, ctx: &Context<'_>, _category_id: Option<i64>) -> async_graphql::Result<Vec<Manga>> {
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

    #[graphql(name = "availableExtensions")]
    async fn available_extensions(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<ExtensionListing>> {
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let result = registry.get_available_extensions().await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(result.into_iter().map(|ext| ext.into()).collect())
    }

    #[graphql(name = "installedExtensions")]
    async fn installed_extensions(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<ExtensionListing>> {
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let installed = registry.get_installed_extensions().await;
        Ok(installed.into_iter().map(|ext| ext.into()).collect())
    }

    #[graphql(name = "extensionRepos")]
    async fn extension_repos(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<ExtensionRepo>> {
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        Ok(registry.repos.iter().cloned().map(|r| r.into()).collect())
    }

    #[graphql(name = "sources")]
    async fn sources(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<Source>> {
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let installed = registry.get_installed_extensions().await;
        
        let mut sources = Vec::new();
        for ext in installed {
            if ext.pkg_name == "eu.kanade.tachiyomi.extension.en.allanime" || ext.pkg_name == "eu.kanade.tachiyomi.extension.all.mangadex" || registry.get_source_by_pkg(&ext.pkg_name).is_some() {
                let id = if ext.pkg_name == "eu.kanade.tachiyomi.extension.en.allanime" {
                    "8861274191478178487".to_string()
                } else if ext.pkg_name == "eu.kanade.tachiyomi.extension.all.mangadex" {
                    "2499283573021221994".to_string()
                } else {
                    "1".to_string() // Simplified for now since we only have one source
                };
                sources.push(Source {
                    id,
                    name: ext.name.clone(),
                    lang: ext.lang.clone(),
                    display_name: format!("{} ({})", ext.name, ext.lang),
                    icon_url: ext.icon_url.clone(),
                    supports_latest: true,
                });
            }
        }
        
        Ok(sources)
    }

    #[graphql(name = "source")]
    async fn source(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<Option<Source>> {
        let sources = self.sources(ctx).await?;
        Ok(sources.into_iter().find(|s| s.id == id))
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
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let sid = source_id.parse::<i64>().map_err(|_| async_graphql::Error::new("Invalid source ID"))?;
        
        let source = registry.get_source(sid)
            .ok_or_else(|| async_graphql::Error::new(format!("Source {} not found", sid)))?;

        let page_res = if query.is_some() || type_name.to_uppercase() == "SEARCH" {
            source.search_manga(query.as_deref().unwrap_or(""), &[], page).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        } else if type_name.to_uppercase() == "LATEST" {
            source.get_latest_updates(page).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        } else {
            source.get_popular_manga(page).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?
        };

        Ok(MangaPagePayload {
            mangas: page_res.manga_list.into_iter().map(Into::into).collect(),
            has_next_page: page_res.has_next_page,
        })
    }

    #[graphql(name = "chapterPages")]
    async fn chapter_pages(&self, ctx: &Context<'_>, chapter_id: i64) -> async_graphql::Result<Vec<Page>> {
        let pool = ctx.data::<SqlitePool>()?;
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        
        let chapter = ChapterRepository::new(pool).get_by_id(chapter_id).await?
            .ok_or_else(|| async_graphql::Error::new(format!("Chapter {} not found", chapter_id)))?;
            
        let manga = MangaRepository::new(pool).get_by_id(chapter.manga_id).await?
            .ok_or_else(|| async_graphql::Error::new(format!("Manga {} not found", chapter.manga_id)))?;
            
        let source = registry.get_source(manga.source_id)
            .ok_or_else(|| async_graphql::Error::new(format!("Source {} not found", manga.source_id)))?;

        let chapter_model: suwayomi_core::models::Chapter = chapter.into();
        let pages = source.get_page_list(&chapter_model).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(pages.into_iter().map(|p| Page {
            index: p.index,
            url: p.url,
            image_url: p.image_url,
        }).collect())
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    #[graphql(name = "installExtension")]
    async fn install_extension(&self, ctx: &Context<'_>, pkg_name: String) -> async_graphql::Result<ExtensionListing> {
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let listing = registry.install_extension(&pkg_name).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(listing.into())
    }

    #[graphql(name = "uninstallExtension")]
    async fn uninstall_extension(&self, ctx: &Context<'_>, pkg_name: String) -> async_graphql::Result<bool> {
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        let result = registry.uninstall_extension(&pkg_name).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(result)
    }

    #[graphql(name = "updateMangaInLibrary")]
    async fn update_manga_in_library(
        &self,
        ctx: &Context<'_>,
        id: i64,
        in_library: bool,
    ) -> async_graphql::Result<Manga> {
        let pool = ctx.data::<SqlitePool>()?;
        let repo = MangaRepository::new(pool);
        repo.update_in_library(id, in_library).await?;
        let manga = repo.get_by_id(id).await?
            .ok_or_else(|| async_graphql::Error::new(format!("Manga {} not found", id)))?;
        Ok(manga.into())
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
    async fn download_chapter(&self, ctx: &Context<'_>, chapter_id: i64) -> async_graphql::Result<bool> {
        let pool = ctx.data::<SqlitePool>()?;
        let chapter_repo = ChapterRepository::new(pool);
        
        let chapter = chapter_repo.get_by_id(chapter_id).await?
            .ok_or_else(|| async_graphql::Error::new(format!("Chapter {} not found", chapter_id)))?;
            
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

    #[graphql(name = "fetchMangaAndChapters")]
    async fn fetch_manga_and_chapters(
        &self,
        ctx: &Context<'_>,
        source_id: String,
        manga_url: String,
    ) -> async_graphql::Result<Manga> {
        let pool = ctx.data::<SqlitePool>()?;
        let registry = ctx.data::<std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>>()?;
        
        let sid = source_id.parse::<i64>().map_err(|_| async_graphql::Error::new("Invalid source ID"))?;
        let source = registry.get_source(sid)
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

        let mut updated_manga = source.get_manga_details(manga).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        if updated_manga.id == 0 {
            let new_id = manga_repo.insert(&updated_manga).await?;
            updated_manga.id = new_id;
        } else {
            manga_repo.update(&updated_manga).await?;
        }

        let mut chapters = source.get_chapter_list(&updated_manga).await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            
        for chapter in &mut chapters {
            chapter.manga_id = updated_manga.id;
        }
        
        ChapterRepository::new(pool).insert_chapters(&chapters).await?;
        
        Ok(updated_manga.into())
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
