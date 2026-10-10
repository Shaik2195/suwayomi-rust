use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::Arc;
use suwayomi_api::{create_router, create_schema, AppState};
use suwayomi_db::{migrations::run_migrations, pool::create_sqlite_pool};
use suwayomi_downloader::queue::DownloadQueue;
use suwayomi_extensions::registry::ExtensionRegistry;
use tower::ServiceExt;

#[tokio::test]
async fn test_sources_graphql() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_sources.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let pool = create_sqlite_pool(&db_url).await.unwrap();

    run_migrations(&pool).await.unwrap();

    let download_queue = DownloadQueue::new();
    let extension_registry = ExtensionRegistry::new(vec![]);

    // We add a mock installed extension manually because we can't easily mock the registry internal state directly
    // Wait, we can't mutate registry.installed directly since it's private.
    // Let's rely on what sources() returns based on `get_installed_extensions`.
    // If it's empty, we might not get sources. Let's see if we can get around this by testing empty or by initializing it.
    // The registry provides `install_extension` but it requires a network call.
    // For now, we will test the empty case or if we can use an internal mock. Let's test the endpoint doesn't crash first.
    let extension_registry = Arc::new(extension_registry);

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

    let query = r#"
    {
        sources {
            id
            name
            lang
        }
    }
    "#;

    let body = serde_json::json!({
        "query": query
    });

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let data = resp_json.get("data").expect("Missing data");
    let sources = data.get("sources").expect("Missing sources");
    assert!(sources.is_array());

    // As we can't easily install a mock extension here without network mocks,
    // it will likely be empty.

    // Test fetchMangaAndChapters with an invalid source to check error handling
    let mutation = r#"
    mutation {
        fetchMangaAndChapters(sourceId: "999", mangaUrl: "/test") {
            id
            title
        }
    }
    "#;

    let body = serde_json::json!({
        "query": mutation
    });

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(resp_json.get("errors").is_some());
}
