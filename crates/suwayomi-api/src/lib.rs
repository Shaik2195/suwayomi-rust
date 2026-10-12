#![allow(clippy::too_many_arguments)]

pub mod graphql;
pub mod middleware;
pub mod rest;
pub mod static_files;
pub mod ws;

use async_graphql::{EmptySubscription, Schema};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::SqlitePool;
use suwayomi_downloader::queue::DownloadQueue;

use crate::graphql::{AppSchema, MutationRoot, QueryRoot};

use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct PageCache {
    pages: Arc<DashMap<i64, Vec<suwayomi_core::models::Page>>>,
    fetch_locks: Arc<DashMap<i64, Arc<Mutex<()>>>>,
}

impl PageCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_or_fetch_pages<F, Fut>(
        &self,
        chapter_id: i64,
        data_dir: &std::path::Path,
        fetch_fn: F,
    ) -> Result<Vec<suwayomi_core::models::Page>, axum::http::StatusCode>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<suwayomi_core::models::Page>, axum::http::StatusCode>>,
    {
        if let Some(pages) = self.pages.get(&chapter_id) {
            return Ok(pages.clone());
        }

        let cache_dir = data_dir.join(format!("cache/pages/{}", chapter_id));
        let pages_json_path = cache_dir.join("pages.json");
        if let Ok(bytes) = tokio::fs::read(&pages_json_path).await {
            if let Ok(pages) = serde_json::from_slice::<Vec<suwayomi_core::models::Page>>(&bytes) {
                self.pages.insert(chapter_id, pages.clone());
                return Ok(pages);
            }
        }

        let lock = self
            .fetch_locks
            .entry(chapter_id)
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .value()
            .clone();

        let _guard = lock.lock().await;

        if let Some(pages) = self.pages.get(&chapter_id) {
            return Ok(pages.clone());
        }

        if let Ok(bytes) = tokio::fs::read(&pages_json_path).await {
            if let Ok(pages) = serde_json::from_slice::<Vec<suwayomi_core::models::Page>>(&bytes) {
                self.pages.insert(chapter_id, pages.clone());
                return Ok(pages);
            }
        }

        let pages = fetch_fn().await?;

        let _ = tokio::fs::create_dir_all(&cache_dir).await;
        if let Ok(json_str) = serde_json::to_string(&pages) {
            let _ = tokio::fs::write(&pages_json_path, json_str.as_bytes()).await;
        }

        self.pages.insert(chapter_id, pages.clone());
        Ok(pages)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub schema: AppSchema,
    pub download_queue: DownloadQueue,
    pub extension_registry: std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>,
    pub data_dir: PathBuf,
    pub page_cache: PageCache,
}

pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/manga/:id", get(rest::get_manga))
        .route("/manga/:id/thumbnail", get(rest::get_manga_thumbnail))
        .route("/manga/:id/chapters", get(rest::get_manga_chapters))
        .route(
            "/manga/:manga_id/chapter/:chapter_id/page/:page",
            get(rest::get_chapter_page),
        )
        .route("/chapter/:id", get(rest::get_chapter))
        .route("/category", get(rest::get_categories))
        .with_state(state.clone());

    let ws_state = state.download_queue.clone();

    Router::new()
        .route(
            "/graphql",
            post(graphql::graphql_handler).get(graphql::graphql_get_handler),
        )
        .route(
            "/api/graphql",
            post(graphql::graphql_handler).get(graphql::graphql_get_handler),
        )
        .route("/ws", get(ws::ws_handler).with_state(ws_state))
        .nest("/api/v1", api_routes)
        .fallback(static_files::static_handler)
        .layer(axum::middleware::from_fn(middleware::basic_auth_middleware))
        .layer(middleware::setup_cors())
        .layer(middleware::setup_tracing())
        .layer(middleware::setup_compression())
        .with_state(state.schema.clone())
}

pub fn create_schema(
    pool: SqlitePool,
    download_queue: DownloadQueue,
    extension_registry: std::sync::Arc<suwayomi_extensions::registry::ExtensionRegistry>,
) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .data(download_queue)
        .data(extension_registry)
        .finish()
}
