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

fn get_referer_header(url: &str) -> Option<&'static str> {
    if url.contains("allmanga") || url.contains("allanime") || url.contains("mkissa") {
        Some("https://allmanga.to")
    } else if url.contains("mangadex") {
        Some("https://mangadex.org")
    } else {
        None
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_referer_header() {
        assert_eq!(
            get_referer_header("https://allmanga.to/image.jpg"),
            Some("https://allmanga.to")
        );
        assert_eq!(
            get_referer_header("https://cdn.allanime.day/image.jpg"),
            Some("https://allmanga.to")
        );
        assert_eq!(
            get_referer_header("https://mkissa.com/image.jpg"),
            Some("https://allmanga.to")
        );
        assert_eq!(
            get_referer_header("https://uploads.mangadex.org/data/123/456.jpg"),
            Some("https://mangadex.org")
        );
        assert_eq!(
            get_referer_header("https://s2.mangadex.network/data/123/456.jpg"),
            Some("https://mangadex.org")
        );
        assert_eq!(get_referer_header("https://example.com/image.jpg"), None);
    }
}

pub async fn get_manga_thumbnail(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, StatusCode> {
    let pool = state.pool;
    let repo = MangaRepository::new(&pool);

    let manga = repo
        .get_by_id(id)
        .await
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
    let mut req = client.get(&thumbnail_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

    if let Some(referer) = get_referer_header(&thumbnail_url) {
        req = req.header("Referer", referer);
    }

    let resp = req.send().await.map_err(|_| StatusCode::BAD_GATEWAY)?;

    if !resp.status().is_success() {
        return Err(StatusCode::BAD_GATEWAY);
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

    let chapter = chapter_repo
        .get_by_id(chapter_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let manga = manga_repo
        .get_by_id(manga_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let dl_path = PathBuf::from(format!(
        "./data/downloads/{}/{}/{}.jpg",
        manga_id, chapter_id, page
    ));
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

    let source = state
        .extension_registry
        .get_source(manga.source_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let pages = source
        .get_page_list(&chapter)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let target_page = pages
        .into_iter()
        .find(|p| p.index == page)
        .ok_or(StatusCode::NOT_FOUND)?;

    let image_url = target_page.image_url.ok_or(StatusCode::NOT_FOUND)?;

    let _ = tokio::fs::create_dir_all(&cache_dir).await;

    let client = reqwest::Client::new();
    let mut req = client.get(&image_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

    if let Some(referer) = get_referer_header(&image_url) {
        req = req.header("Referer", referer);
    }

    let resp = req.send().await;

    let mut success_resp = match resp {
        Ok(r) if r.status().is_success() => Some(r),
        _ => None,
    };

    if success_resp.is_none() && image_url.contains("mangadex.network") {
        if let Some(idx) = image_url.find("/data") {
            let fallback_url = format!("https://uploads.mangadex.org{}", &image_url[idx..]);
            let mut fallback_req = client.get(&fallback_url)
                .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36");

            if let Some(referer) = get_referer_header(&fallback_url) {
                fallback_req = fallback_req.header("Referer", referer);
            }

            if let Ok(r) = fallback_req.send().await {
                if r.status().is_success() {
                    success_resp = Some(r);
                }
            }
        }
    }

    let resp = success_resp.ok_or(StatusCode::BAD_GATEWAY)?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
