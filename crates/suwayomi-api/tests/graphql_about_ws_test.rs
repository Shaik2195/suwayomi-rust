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
async fn test_graphql_about_parity() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_about.db");
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

    let query = r#"
    query GetAbout {
        aboutServer {
            name
            version
            buildTime
            buildType
            discord
            github
            platformInfo {
                arch
                headless
                os {
                    name
                    build
                    version
                }
                jvm {
                    javaVersion
                    vmName
                    vmVendor
                    vmVersion
                }
            }
        }
        aboutWebUI {
            channel
            tag
            updateTimestamp
        }
    }
    "#;

    let body = serde_json::json!({
        "query": query
    });

    let request = Request::builder()
        .uri("/api/graphql")
        .method("POST")
        .header("host", "localhost")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    println!("Status: {}, Response body: {}", status, body_str);

    assert_eq!(status, StatusCode::OK);
    let resp_json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(resp_json.get("errors").is_none(), "Errors: {:?}", resp_json.get("errors"));

    let data = resp_json.get("data").expect("Missing data");

    let about_server = data.get("aboutServer").expect("Missing aboutServer");
    assert_eq!(about_server["name"], "Suwayomi-Rust");

    let platform_info = about_server.get("platformInfo").expect("Missing platformInfo");
    assert_eq!(platform_info["arch"], std::env::consts::ARCH);
    assert_eq!(platform_info["headless"], false);

    let os = platform_info.get("os").expect("Missing os");
    assert_eq!(os["name"], std::env::consts::OS);

    let jvm = platform_info.get("jvm").expect("Missing jvm");
    assert_eq!(jvm["javaVersion"], "21.0.0 (Native)");
    assert_eq!(jvm["vmName"], "Suwayomi-Rust Tokio Engine");
    assert_eq!(jvm["vmVendor"], "Suwayomi-Rust");
    assert_eq!(jvm["vmVersion"], "0.1.0");

    let about_webui = data.get("aboutWebUI").expect("Missing aboutWebUI");
    assert_eq!(about_webui["channel"], "stable");
    assert_eq!(about_webui["tag"], "latest");
    assert_eq!(about_webui["updateTimestamp"], "unknown");
}

#[tokio::test]
async fn test_graphql_websocket_upgrade() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("test_ws.db");
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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    for endpoint in ["/graphql", "/api/graphql"] {
        let ws_url = format!("ws://{}{} ", addr, endpoint);
        let ws_url = ws_url.trim();

        let req = tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(ws_url)
            .header("Host", addr.to_string())
            .header("Upgrade", "websocket")
            .header("Connection", "Upgrade")
            .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Protocol", "graphql-transport-ws")
            .body(())
            .unwrap();

        let (mut socket, response) = tokio_tungstenite::connect_async(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS, "Failed for endpoint {}", endpoint);
        let _ = socket.close(None).await;
    }
}
