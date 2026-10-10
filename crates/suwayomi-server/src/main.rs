use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    /// Port to bind to
    #[arg(short, long, default_value_t = 4567)]
    port: u16,

    /// Data directory
    #[arg(long, default_value = "/data")]
    data_dir: String,

    /// Config file path
    #[arg(long)]
    config: Option<String>,
}

use std::path::PathBuf;
use suwayomi_api::AppState;
use tokio::signal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    tracing::info!("Suwayomi Server starting up...");

    // Create data directory if not present
    let data_dir = PathBuf::from(&args.data_dir);
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }

    // Initialize SQLite pool
    let db_path = data_dir.join("suwayomi.db");
    let database_url = format!("sqlite://{}", db_path.to_string_lossy());
    let pool = suwayomi_db::pool::create_sqlite_pool(&database_url).await?;

    // Run migrations
    suwayomi_db::migrations::run_migrations(&pool).await?;

    // Configure Extension Registry
    let repos = vec![suwayomi_extensions::types::ExtensionRepo {
        name: "Keiyoushi".to_string(),
        url: "https://raw.githubusercontent.com/keiyoushi/extensions/repo/index.pb".to_string(),
    }];
    let extension_registry =
        std::sync::Arc::new(suwayomi_extensions::registry::ExtensionRegistry::new(repos));

    // Build API router
    let download_queue = suwayomi_downloader::queue::DownloadQueue::new();
    let schema = suwayomi_api::create_schema(
        pool.clone(),
        download_queue.clone(),
        extension_registry.clone(),
    );
    let state = AppState {
        pool: pool.clone(),
        schema,
        download_queue: download_queue.clone(),
        extension_registry,
    };
    let app = suwayomi_api::create_router(state);

    // Start background download workers
    let storage = suwayomi_downloader::storage::ChapterStorage::new(data_dir.join("downloads"));

    // We instantiate a generic JsExtensionRuntime and JsMangaSource just to satisfy the downloader worker pool compilation
    // In a real application, the source instance is retrieved dynamically per chapter/manga based on source_id
    let runtime = suwayomi_extensions::runtime::JsExtensionRuntime::new();
    let dummy_source =
        std::sync::Arc::new(suwayomi_extensions::runtime::JsMangaSource::new(runtime));

    let worker_pool = suwayomi_downloader::worker::DownloadWorkerPool::new(
        download_queue.clone(),
        storage,
        std::sync::Arc::new(pool),
        dummy_source,
    );
    worker_pool.start(4).await;

    // Bind tokio::net::TcpListener
    let addr = format!("{}:{}", args.bind, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    // Serve Axum router with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("Signal received, starting graceful shutdown");
}
