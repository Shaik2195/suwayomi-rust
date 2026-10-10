use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse},
};
use rust_embed::RustEmbed;
use std::env;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.is_empty() {
        path = "index.html".to_string();
    }

    // Security check: Prevent directory traversal
    if path.contains("..") {
        return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    // Check if WEBUI_DIR is set and serve from there if the file exists
    if let Ok(webui_dir) = env::var("WEBUI_DIR") {
        let mut fs_path = PathBuf::from(&webui_dir);
        fs_path.push(&path);

        if fs_path.exists() {
            if let Ok(content) = tokio::fs::read(&fs_path).await {
                let mime = mime_guess::from_path(&fs_path).first_or_octet_stream();
                return ([(header::CONTENT_TYPE, mime.as_ref())], content).into_response();
            }
        }

        // If it's a SPA fallback and file wasn't found on disk, try index.html on disk
        let mut index_path = PathBuf::from(&webui_dir);
        index_path.push("index.html");
        if index_path.exists() {
            if let Ok(content) = tokio::fs::read(&index_path).await {
                return Html(content).into_response();
            }
        }
    }

    // Lookup in embedded assets
    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // SPA fallback to index.html
            match Assets::get("index.html") {
                Some(content) => Html(content.data).into_response(),
                None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
            }
        }
    }
}
