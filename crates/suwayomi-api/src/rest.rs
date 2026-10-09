use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sqlx::SqlitePool;

use suwayomi_db::repositories::{CategoryRepository, ChapterRepository, MangaRepository};

pub async fn get_manga(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let repo = MangaRepository::new(&pool);
    match repo.get_by_id(id).await {
        Ok(Some(manga)) => Ok(Json(manga)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_manga_thumbnail(
    State(_pool): State<SqlitePool>,
    Path(_id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    // Return a placeholder for now
    let placeholder = vec![];
    Ok((
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        placeholder,
    ))
}

pub async fn get_chapter_page(
    State(_pool): State<SqlitePool>,
    Path((_manga_id, _chapter_id, _page)): Path<(i64, i64, i32)>,
) -> Result<impl IntoResponse, StatusCode> {
    // Return a placeholder for now
    let placeholder = vec![];
    Ok((
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        placeholder,
    ))
}

pub async fn get_manga_chapters(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let repo = ChapterRepository::new(&pool);
    match repo.get_by_manga_id(id).await {
        Ok(chapters) => Ok(Json(chapters)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_chapter(
    State(pool): State<SqlitePool>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let repo = ChapterRepository::new(&pool);
    match repo.get_by_id(id).await {
        Ok(Some(chapter)) => Ok(Json(chapter)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_categories(
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, StatusCode> {
    let repo = CategoryRepository::new(&pool);
    match repo.get_categories().await {
        Ok(categories) => Ok(Json(categories)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
