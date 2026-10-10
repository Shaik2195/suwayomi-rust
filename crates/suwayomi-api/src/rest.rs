use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

use suwayomi_db::repositories::{CategoryRepository, ChapterRepository, MangaRepository};

use crate::AppState;

pub async fn get_manga(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = state.pool;
    let repo = MangaRepository::new(&pool);
    match repo.get_by_id(id).await {
        Ok(Some(manga)) => Ok(Json(manga)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_manga_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = state.pool;
    let repo = MangaRepository::new(&pool);
    
    let manga = repo.get_by_id(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let thumbnail_url = manga.thumbnail_url.ok_or(StatusCode::NOT_FOUND)?;

    let cache_dir = PathBuf::from("./data/cache/thumbnails");
    let _ = tokio::fs::create_dir_all(&cache_dir).await;
    let cache_file = cache_dir.join(id.to_string());

    if let Ok(bytes) = tokio::fs::read(&cache_file).await {
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
            bytes,
        ));
    }

    let client = reqwest::Client::new();
    let resp = client.get(&thumbnail_url)
        .header("Referer", "https://allmanga.to")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    let bytes = resp.bytes().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Ok(mut file) = tokio::fs::File::create(&cache_file).await {
        let _ = file.write_all(&bytes).await;
    }

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
        bytes.to_vec(),
    ))
}

pub async fn get_chapter_page(
    State(state): State<AppState>,
    Path((manga_id, chapter_id, page)): Path<(i64, i64, i32)>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = state.pool;
    let chapter_repo = ChapterRepository::new(&pool);
    let manga_repo = MangaRepository::new(&pool);

    let chapter = chapter_repo.get_by_id(chapter_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let manga = manga_repo.get_by_id(manga_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let dl_path = PathBuf::from(format!("./data/downloads/{}/{}/{}.jpg", manga_id, chapter_id, page));
    if let Ok(bytes) = tokio::fs::read(&dl_path).await {
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
            bytes,
        ));
    }

    let cache_dir = PathBuf::from(format!("./data/cache/pages/{}", chapter_id));
    let cache_file = cache_dir.join(page.to_string());
    if let Ok(bytes) = tokio::fs::read(&cache_file).await {
        return Ok((
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
            bytes,
        ));
    }

    let source = state.extension_registry.get_source(manga.source_id).ok_or(StatusCode::NOT_FOUND)?;
    
    let pages = source.get_page_list(&chapter).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let target_page = pages.into_iter()
        .find(|p| p.index == page)
        .ok_or(StatusCode::NOT_FOUND)?;

    let image_url = target_page.image_url.ok_or(StatusCode::NOT_FOUND)?;

    let _ = tokio::fs::create_dir_all(&cache_dir).await;

    let client = reqwest::Client::new();
    let resp = client.get(&image_url)
        .header("Referer", "https://allmanga.to")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    let bytes = resp.bytes().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if let Ok(mut file) = tokio::fs::File::create(&cache_file).await {
        let _ = file.write_all(&bytes).await;
    }

    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/jpeg")],
        bytes.to_vec(),
    ))
}

pub async fn get_manga_chapters(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = state.pool;
    let repo = ChapterRepository::new(&pool);
    match repo.get_by_manga_id(id).await {
        Ok(chapters) => Ok(Json(chapters)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_chapter(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = state.pool;
    let repo = ChapterRepository::new(&pool);
    match repo.get_by_id(id).await {
        Ok(Some(chapter)) => Ok(Json(chapter)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_categories(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = state.pool;
    let repo = CategoryRepository::new(&pool);
    match repo.get_categories().await {
        Ok(categories) => Ok(Json(categories)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
