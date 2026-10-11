use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::Arc;
use suwayomi_api::{create_router, create_schema, AppState};
use suwayomi_core::models::{Manga, MangaStatus};
use suwayomi_db::{
    migrations::run_migrations, pool::create_sqlite_pool, repositories::MangaRepository,
};
use suwayomi_downloader::queue::DownloadQueue;
use suwayomi_extensions::registry::ExtensionRegistry;
use tower::ServiceExt;

#[tokio::test]
async fn test_graphql_metas_and_sources() {
    let pool = create_sqlite_pool("sqlite::memory:")
        .await
        .expect("Failed to create pool");
    run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let download_queue = DownloadQueue::new();
    let extension_registry = Arc::new(ExtensionRegistry::new(vec![]));

    let schema = create_schema(
        pool.clone(),
        download_queue.clone(),
        extension_registry.clone(),
    );

    let state = AppState {
        pool: pool.clone(),
        schema,
        download_queue,
        extension_registry,
        data_dir: std::path::PathBuf::from("./data"),
        page_cache: suwayomi_api::PageCache::new(),
    };

    let app = create_router(state);

    let manga_repo = MangaRepository::new(&pool);
    let manga_id = manga_repo
        .insert(&Manga {
            id: 0,
            source_id: 2499283573021221994,
            url: "/manga/test".to_string(),
            title: "Test Meta Manga".to_string(),
            artist: None,
            author: None,
            description: None,
            genre: None,
            status: MangaStatus::Ongoing,
            thumbnail_url: None,
            update_strategy: 0,
            initialized: true,
        })
        .await
        .expect("Failed to insert manga");

    // 1. Test setMangaMetas mutation
    let set_meta_query = format!(
        r#"{{"query": "mutation {{ setMangaMetas(input: {{ items: [{{ mangaIds: [{}], metas: [{{ key: \"reading_mode\", value: \"webtoon\" }}] }}] }}) {{ metas {{ mangaId key value }} }} }}"}}"#,
        manga_id
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(set_meta_query))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("reading_mode"));
    assert!(body_str.contains("webtoon"));

    // 2. Test query manga metas
    let query_manga = format!(
        r#"{{"query": "query {{ manga(id: {}) {{ id title meta {{ key value }} }} }}"}}"#,
        manga_id
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(query_manga))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("reading_mode"));

    // 3. Test deleteMangaMetas mutation
    let delete_meta_query = format!(
        r#"{{"query": "mutation {{ deleteMangaMetas(input: {{ items: [{{ mangaIds: [{}], keys: [\"reading_mode\"] }}] }}) {{ metas {{ mangaId key value }} }} }}"}}"#,
        manga_id
    );

    let req = Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(delete_meta_query))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. Test sources query returning native extensions
    let sources_query = r#"{"query": "query { sources { nodes { id name lang } } }"}"#;

    let req = Request::builder()
        .method("POST")
        .uri("/api/graphql")
        .header("content-type", "application/json")
        .body(Body::from(sources_query))
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("MangaDex") || body_str.contains("AllAnime"));
}
