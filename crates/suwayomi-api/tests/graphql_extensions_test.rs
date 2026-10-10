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
use suwayomi_extensions::types::ExtensionRepo;
use tower::ServiceExt;

#[tokio::test]
async fn test_extensions_graphql() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_ext.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let pool = create_sqlite_pool(&db_url).await.unwrap();

    run_migrations(&pool).await.unwrap();

    let download_queue = DownloadQueue::new();
    let extension_registry = Arc::new(ExtensionRegistry::new(vec![ExtensionRepo {
        name: "Mock Repo".to_string(),
        url: "http://localhost:0/mock.pb".to_string(), // Invalid on purpose, we just test the schema
    }]));
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
        availableExtensions {
            name
        }
        installedExtensions {
            name
        }
        extensionRepos {
            name
            url
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

    let available = data
        .get("availableExtensions")
        .expect("Missing availableExtensions");
    assert!(available.is_array()); // Might be empty because the repo fetch fails, but it shouldn't error out the whole query.

    let installed = data
        .get("installedExtensions")
        .expect("Missing installedExtensions");
    assert!(installed.is_array());
    assert_eq!(installed.as_array().unwrap().len(), 0);

    let repos = data.get("extensionRepos").expect("Missing extensionRepos");
    assert!(repos.is_array());
    assert_eq!(repos.as_array().unwrap().len(), 1);
    assert_eq!(repos[0]["name"], "Mock Repo");

    // Test install mutation (should fail because repo is invalid, but schema matches)
    let mutation = r#"
    mutation {
        installExtension(pkgName: "eu.kanade.tachiyomi.extension.en.mock") {
            pkgName
            name
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

    // Test uninstall mutation (should fail/return false because it's not installed)
    let mutation = r#"
    mutation {
        uninstallExtension(pkgName: "eu.kanade.tachiyomi.extension.en.mock")
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

    // We expect successful execution of the mutation but it returns true even if nothing was uninstalled.
    let data = resp_json
        .get("data")
        .expect("Missing data in uninstall mutation");
    assert_eq!(
        data.get("uninstallExtension").unwrap().as_bool().unwrap(),
        true
    );
}
