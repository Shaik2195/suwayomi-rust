use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use suwayomi_api::{create_router, create_schema, AppState};
use suwayomi_db::{migrations::run_migrations, pool::create_sqlite_pool};
use suwayomi_downloader::queue::DownloadQueue;
use tower::ServiceExt;

#[tokio::test]
async fn test_spa_static_files_fallback() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let pool = create_sqlite_pool(&db_url).await.unwrap();

    run_migrations(&pool).await.unwrap();

    let download_queue = DownloadQueue::new();
    let extension_registry = std::sync::Arc::new(
        suwayomi_extensions::registry::ExtensionRegistry::new(vec![]),
    );
    let schema = create_schema(
        pool.clone(),
        download_queue.clone(),
        extension_registry.clone(),
    );

    let state = AppState {
        pool,
        schema,
        download_queue,
        extension_registry,
    };

    let app = create_router(state);

    // Test GET / returns 200 with text/html
    let request = Request::builder().uri("/").body(Body::empty()).unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/html");

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<title>Suwayomi"));

    // Test GET /library returns 200 with text/html (SPA fallback)
    let request = Request::builder()
        .uri("/library")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/html; charset=utf-8" // Html() wrapper adds charset
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("<title>Suwayomi"));

    // Test API route works and is not intercepted by fallback
    let request = Request::builder()
        .uri("/api/v1/category")
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
