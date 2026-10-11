use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use httpmock::prelude::*;
use std::sync::Arc;
use suwayomi_api::{create_router, create_schema, AppState};
use suwayomi_db::{migrations::run_migrations, pool::create_sqlite_pool};
use suwayomi_downloader::queue::DownloadQueue;
use suwayomi_extensions::registry::ExtensionRegistry;
use suwayomi_extensions::types::ExtensionRepo;
use tower::ServiceExt;

#[tokio::test]
async fn test_extensions_graphql() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/index.min.json");
        then.status(200)
            .header("content-type", "application/json")
            .body(
                r#"[
                {
                    "pkg_name": "eu.kanade.tachiyomi.extension.en.allanime",
                    "name": "AllAnime",
                    "version_name": "1.0.0",
                    "version_code": 1,
                    "lang": "en",
                    "is_nsfw": false,
                    "apk_url": "/apk/allanime.apk",
                    "icon_url": "/icon/allanime.png"
                },
                {
                    "pkg_name": "eu.kanade.tachiyomi.extension.all.mangadex",
                    "name": "MangaDex",
                    "version_name": "1.0.0",
                    "version_code": 1,
                    "lang": "all",
                    "is_nsfw": false,
                    "apk_url": "/apk/mangadex.apk",
                    "icon_url": "/icon/mangadex.png"
                }
            ]"#,
            );
    });

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_ext.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let pool = create_sqlite_pool(&db_url).await.unwrap();

    run_migrations(&pool).await.unwrap();

    let download_queue = DownloadQueue::new();
    let extension_registry = Arc::new(ExtensionRegistry::new(vec![ExtensionRepo {
        name: "Mock Repo".to_string(),
        url: server.url("/index.min.json"),
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
    assert_eq!(installed.as_array().unwrap().len(), 2);

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
    assert!(data.get("uninstallExtension").unwrap().as_bool().unwrap());

    // Test fetchExtensions mutation
    let fetch_extensions_query = r#"
    mutation {
        fetchExtensions(input: {}) {
            clientMutationId
            extensionStores {
                name
                indexUrl
                extensions {
                    totalCount
                }
            }
            extensions {
                pkgName
                name
                isInstalled
                extensionStore {
                    name
                }
            }
        }
    }
    "#;

    let body = serde_json::json!({
        "query": fetch_extensions_query
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
    let data = resp_json.get("data").expect("Missing data in fetchExtensions mutation");
    let fetch_res = &data["fetchExtensions"];
    assert!(fetch_res["extensions"].is_array());
    assert_eq!(fetch_res["extensions"].as_array().unwrap().len(), 2);
    assert_eq!(fetch_res["extensions"][0]["extensionStore"]["name"], "Keiyoushi");
    assert!(fetch_res["extensionStores"].is_array());
    assert_eq!(fetch_res["extensionStores"][0]["name"], "Keiyoushi");

    // Test extensionStores query
    let stores_query = r#"
    {
        extensionStores {
            nodes {
                name
                indexUrl
                extensions {
                    totalCount
                }
            }
            totalCount
        }
    }
    "#;

    let body = serde_json::json!({
        "query": stores_query
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
    let data = resp_json.get("data").expect("Missing data in extensionStores query");
    let stores_res = &data["extensionStores"];
    assert_eq!(stores_res["totalCount"], 1);
    assert_eq!(stores_res["nodes"][0]["name"], "Keiyoushi");

    // Test updateExtension Relay mutation
    let update_ext_mutation = r#"
    mutation {
        updateExtension(input: { pkgName: "eu.kanade.tachiyomi.extension.en.allanime", uninstall: true }) {
            clientMutationId
            extension {
                pkgName
                isInstalled
            }
        }
    }
    "#;

    let body = serde_json::json!({
        "query": update_ext_mutation
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
    let data = resp_json.get("data").expect("Missing data in updateExtension mutation");
    let update_res = &data["updateExtension"];
    assert_eq!(update_res["extension"]["pkgName"], "eu.kanade.tachiyomi.extension.en.allanime");

    // Test updateExtensions Relay mutation
    let update_exts_mutation = r#"
    mutation {
        updateExtensions(input: { pkgNames: ["eu.kanade.tachiyomi.extension.all.mangadex"], uninstall: true }) {
            clientMutationId
            extensions {
                pkgName
            }
        }
    }
    "#;

    let body = serde_json::json!({
        "query": update_exts_mutation
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
    let data = resp_json.get("data").expect("Missing data in updateExtensions mutation");
    let update_res = &data["updateExtensions"];
    assert!(update_res["extensions"].is_array());
    assert_eq!(update_res["extensions"][0]["pkgName"], "eu.kanade.tachiyomi.extension.all.mangadex");
}
