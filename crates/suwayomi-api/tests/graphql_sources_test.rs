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
            nodes {
                id
                name
                lang
            }
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
    assert!(sources.get("nodes").unwrap().is_array());

    // As we can't easily install a mock extension here without network mocks,
    // it will likely be empty.

    // Test fetchMangaAndChapters with an invalid source to check error handling
    let mutation = r#"
    mutation {
        fetchMangaAndChapters(input: { id: 999 }) {
            manga {
                id
                title
            }
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

#[tokio::test]
async fn test_source_manga_and_metas_graphql() {
    let server = httpmock::MockServer::start_async().await;

    // Mock MangaDex popular manga response
    let popular_response = serde_json::json!({
        "result": "ok",
        "response": "collection",
        "data": [
            {
                "id": "manga-1",
                "type": "manga",
                "attributes": {
                    "title": { "en": "Test MangaDex Manga" },
                    "description": { "en": "Test Description" },
                    "status": "ongoing",
                    "contentRating": "safe"
                },
                "relationships": [
                    {
                        "id": "cover-1",
                        "type": "cover_art",
                        "attributes": {
                            "fileName": "cover1.jpg"
                        }
                    }
                ]
            }
        ],
        "limit": 10,
        "offset": 0,
        "total": 1
    });

    server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/manga")
            .query_param("order[followedCount]", "desc");
        then.status(200)
            .header("content-type", "application/json")
            .body(popular_response.to_string());
    });

    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_source_manga.db");
    let db_url = format!("sqlite://{}", db_path.to_str().unwrap());

    let pool = create_sqlite_pool(&db_url).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let download_queue = DownloadQueue::new();
    let extension_registry = Arc::new(ExtensionRegistry::new(vec![]));

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

    // 1. fetchSourceManga with POPULAR
    let fetch_mutation = r#"
    mutation {
        fetchSourceManga(input: {
            sourceId: "2499283573021221994",
            type: POPULAR,
            page: 1
        }) {
            hasNextPage
            mangas {
                id
                title
                inLibrary
                sourceId
            }
        }
    }
    "#;

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": fetch_mutation })).unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let fetch_data = resp_json
        .get("data")
        .expect("Missing data in fetchSourceManga")
        .get("fetchSourceManga")
        .unwrap();

    let mangas = fetch_data.get("mangas").unwrap().as_array().unwrap();
    assert!(!mangas.is_empty());
    let inserted_id = mangas[0].get("id").unwrap().as_i64().unwrap();
    assert!(inserted_id > 0);

    // 2. Query manga(id: ...) to confirm it was persisted
    let query_manga = format!(
        r#"
        query {{
            manga(id: {}) {{
                id
                title
                inLibrary
            }}
        }}
        "#,
        inserted_id
    );

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": query_manga })).unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let manga_data = resp_json.get("data").unwrap().get("manga").unwrap();
    assert_eq!(manga_data.get("id").unwrap().as_i64().unwrap(), inserted_id);
    assert_eq!(
        manga_data.get("inLibrary").unwrap().as_bool().unwrap(),
        false
    );

    // 3. updateManga inLibrary: true
    let update_manga_mutation = format!(
        r#"
        mutation {{
            updateManga(input: {{ id: {}, patch: {{ inLibrary: true }} }}) {{
                manga {{
                    id
                    inLibrary
                }}
            }}
        }}
        "#,
        inserted_id
    );

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": update_manga_mutation })).unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let updated_manga = resp_json
        .get("data")
        .unwrap()
        .get("updateManga")
        .unwrap()
        .get("manga")
        .unwrap();
    assert_eq!(
        updated_manga.get("inLibrary").unwrap().as_bool().unwrap(),
        true
    );

    // 4. Source Metas Mutations: setSourceMetas and deleteSourceMetas
    let set_metas_mutation = r#"
    mutation {
        setSourceMetas(input: {
            items: [
                {
                    sourceIds: ["2499283573021221994"],
                    metas: [{ key: "PINNED", value: "true" }]
                }
            ]
        }) {
            metas {
                sourceId
                key
                value
            }
        }
    }
    "#;

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": set_metas_mutation })).unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let set_metas = resp_json
        .get("data")
        .unwrap()
        .get("setSourceMetas")
        .unwrap()
        .get("metas")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(set_metas.len(), 1);
    assert_eq!(set_metas[0].get("key").unwrap().as_str().unwrap(), "PINNED");
    assert_eq!(set_metas[0].get("value").unwrap().as_str().unwrap(), "true");

    let delete_metas_mutation = r#"
    mutation {
        deleteSourceMetas(input: {
            items: [
                {
                    sourceIds: ["2499283573021221994"],
                    keys: ["PINNED"]
                }
            ]
        }) {
            metas {
                sourceId
                key
                value
            }
        }
    }
    "#;

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": delete_metas_mutation })).unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 5. Test setSourceMeta, deleteSourceMeta, and updateSourcePreference
    let set_single_meta_mutation = r#"
    mutation {
        setSourceMeta(input: {
            sourceId: "2499283573021221994",
            key: "TEST_KEY",
            value: "TEST_VAL"
        }) {
            meta {
                sourceId
                key
                value
            }
        }
    }
    "#;

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": set_single_meta_mutation })).unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    let single_meta = resp_json
        .get("data")
        .unwrap()
        .get("setSourceMeta")
        .unwrap()
        .get("meta")
        .unwrap();
    assert_eq!(
        single_meta.get("key").unwrap().as_str().unwrap(),
        "TEST_KEY"
    );
    assert_eq!(
        single_meta.get("value").unwrap().as_str().unwrap(),
        "TEST_VAL"
    );

    let delete_single_meta_mutation = r#"
    mutation {
        deleteSourceMeta(input: {
            sourceId: "2499283573021221994",
            key: "TEST_KEY"
        }) {
            clientMutationId
        }
    }
    "#;

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": delete_single_meta_mutation }))
                .unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let update_pref_mutation = r#"
    mutation {
        updateSourcePreference(input: {
            sourceId: "2499283573021221994"
        }) {
            source {
                id
            }
        }
    }
    "#;

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({ "query": update_pref_mutation })).unwrap(),
        ))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
