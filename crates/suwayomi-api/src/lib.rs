pub mod graphql;
pub mod rest;
pub mod middleware;
pub mod ws;

use async_graphql::{EmptySubscription, Schema};
use axum::{
    routing::{get, post},
    Router,
};
use sqlx::SqlitePool;

use crate::graphql::{AppSchema, MutationRoot, QueryRoot};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub schema: AppSchema,
}

pub fn create_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/manga/:id", get(rest::get_manga))
        .route("/manga/:id/thumbnail", get(rest::get_manga_thumbnail))
        .route("/manga/:id/chapters", get(rest::get_manga_chapters))
        .route("/manga/:manga_id/chapter/:chapter_id/page/:page", get(rest::get_chapter_page))
        .route("/chapter/:id", get(rest::get_chapter))
        .route("/category", get(rest::get_categories))
        .with_state(state.pool.clone());

    Router::new()
        .route("/graphql", post(graphql::graphql_handler).get(graphql::graphql_playground))
        .route("/api/graphql", post(graphql::graphql_handler).get(graphql::graphql_playground))
        .route("/ws", get(ws::ws_handler))
        .nest("/api/v1", api_routes)
        .layer(middleware::setup_cors())
        .layer(middleware::setup_tracing())
        .layer(middleware::setup_compression())
        .with_state(state.schema.clone())
}

pub fn create_schema(pool: SqlitePool) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .finish()
}
