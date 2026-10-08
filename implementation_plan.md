# Suwayomi-Server → Rust Full Rewrite

A complete, high-performance rewrite of [Suwayomi-Server](https://github.com/Suwayomi/Suwayomi-Server) (the canonical headless Tachiyomi/Mihon manga server used with Docker) in **Rust**, targeting maximum throughput, minimal memory footprint, and Docker-native deployment.

## Background

### Original Stack (Suwayomi-Server / Tachidesk)
| Layer | Technology |
|---|---|
| Language | Kotlin (~33%) + Java (~66%) |
| Runtime | JVM (OpenJDK 11+) |
| Web Server | Javalin (Jetty-based) |
| API | GraphQL (graphql-java) + REST |
| Database | Embedded H2 (SQL) |
| Extension Runtime | APK → Dex2Jar → ASM repair → android.* stubs → child-first ClassLoader isolation |
| Build | Gradle multi-module |
| Config | HOCON (server.conf) |
| Docker | Multi-stage JVM image, ~400MB+ |

### Performance Bottlenecks Identified
1. **JVM Cold Start**: 3-8s startup, high RSS baseline (~200-400MB idle)
2. **GC Pauses**: Stop-the-world GC during bulk chapter downloads
3. **Extension Sandbox**: Dex2Jar conversion + ASM passes are CPU-intensive at startup
4. **Blocking I/O**: H2 + Javalin thread-per-request model limits concurrency under load
5. **Image Pipeline**: No zero-copy streaming; images buffered fully in heap memory
6. **Docker Image Size**: JVM base images are 200MB+; total image 400-600MB

### Target Rust Stack
| Layer | Technology | Rationale |
|---|---|---|
| Async Runtime | `tokio` 1.x | Industry standard, work-stealing scheduler |
| Web Framework | `axum` 0.8+ | Type-safe, tower middleware, built on hyper |
| GraphQL | `async-graphql` 7.x | Native async, schema-first or derive macros |
| Database | `sqlx` 0.8+ (SQLite) | Compile-time query verification, async, migrations |
| HTTP Client | `reqwest` 0.12+ | Connection pooling, TLS, async streaming |
| Image Processing | `image` + `turbojpeg` (optional) | Zero-copy decode/transcode |
| Extension Runtime | `boa_engine` (JS) or `wasmtime` (Wasm) | Sandboxed extension execution |
| Serialization | `serde` + `serde_json` | Zero-cost abstraction |
| Config | `config` crate (TOML/HOCON) | Layered config with env override |
| Logging | `tracing` + `tracing-subscriber` | Structured, async-safe logging |
| CLI | `clap` 4.x | Derive-based argument parsing |
| Docker | `FROM scratch` or `alpine:3.x` | Statically linked binary, 15-30MB image |

### Performance Targets
| Metric | Suwayomi (JVM) | Target (Rust) |
|---|---|---|
| Cold Start | 3-8s | <100ms |
| Idle RSS | 200-400MB | 10-30MB |
| Concurrent Downloads | ~20 (thread pool) | 1000+ (async tasks) |
| Image Serve Latency (p99) | ~50ms | <5ms (sendfile/zero-copy) |
| Docker Image Size | 400-600MB | 15-30MB |
| GC Pauses | 10-100ms | 0 (no GC) |

---

## Proposed Changes

### Phase 1: Foundation (Jules Sessions 1-3)

#### [NEW] Project Scaffold & Configuration

Create the Cargo workspace with the following crate layout:

```
suwayomi-rust/
├── Cargo.toml              # Workspace root
├── Cargo.lock
├── .github/
│   └── workflows/
│       ├── ci.yml          # Build + test + clippy + fmt
│       └── docker.yml      # Multi-arch Docker build
├── config/
│   └── default.toml        # Default server configuration
├── migrations/
│   └── 001_initial.sql     # SQLite schema
├── crates/
│   ├── suwayomi-core/      # Domain models, traits, error types
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models/     # Manga, Chapter, Source, Extension, Category, etc.
│   │       ├── error.rs    # Unified error enum with thiserror
│   │       └── traits.rs   # Source, Downloader, Storage trait definitions
│   ├── suwayomi-db/        # Database layer (SQLx + SQLite)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pool.rs     # Connection pool setup
│   │       ├── migrations.rs
│   │       └── repositories/ # MangaRepo, ChapterRepo, CategoryRepo, etc.
│   ├── suwayomi-extensions/ # Extension loading & sandboxed runtime
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── registry.rs  # Extension repository index
│   │       ├── loader.rs    # Download, verify, load extensions
│   │       ├── runtime.rs   # JS/Wasm sandbox execution
│   │       └── types.rs     # ExtensionInfo, FilterList, etc.
│   ├── suwayomi-downloader/ # Download queue & chapter fetcher
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── queue.rs     # Priority queue with pause/resume/reorder
│   │       ├── worker.rs    # Async download workers
│   │       └── storage.rs   # Disk layout: Manga/Chapter/pages
│   ├── suwayomi-api/        # Axum + async-graphql API server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── graphql/     # Schema, queries, mutations, subscriptions
│   │       ├── rest/        # Legacy REST endpoints for compatibility
│   │       ├── middleware/   # Auth, CORS, compression, rate limiting
│   │       └── ws.rs        # WebSocket for download progress
│   └── suwayomi-server/     # Binary entry point
│       ├── Cargo.toml
│       └── src/
│           └── main.rs      # CLI args, config loading, server startup
├── Dockerfile              # Multi-stage: build → scratch/alpine
├── docker-compose.yml      # Dev/prod compose with volumes
└── README.md
```

---

#### [NEW] `suwayomi-core` — Domain Models & Traits

Core domain types shared across all crates:

| Model | Fields | Notes |
|---|---|---|
| `Manga` | id, source_id, url, title, artist, author, description, genre, status, thumbnail_url, initialized, in_library, categories | Mapped from Tachiyomi's `SManga` |
| `Chapter` | id, manga_id, url, name, date_upload, chapter_number, scanlator, read, bookmarked, last_page_read, downloaded | Mapped from `SChapter` |
| `Source` | id, name, lang, icon_url, supports_latest, is_nsfw | Extension source metadata |
| `Extension` | pkg_name, name, version_name, version_code, lang, is_nsfw, has_update, obsolete, icon_url, sources[] | Extension package info |
| `Category` | id, name, order, default | Library categories |
| `Track` | id, manga_id, tracker_id, remote_id, title, status, score, total_chapters, last_chapter_read | Tracker integrations |
| `DownloadItem` | chapter_id, manga_id, state, progress, error | Download queue entry |
| `ServerConfig` | bind_ip, port, socks_proxy, basic_auth, download_path, max_parallel_downloads, extension_repos[] | TOML config |

Key traits:
```rust
#[async_trait]
pub trait MangaSource: Send + Sync {
    async fn get_popular_manga(&self, page: u32) -> Result<MangaPage>;
    async fn get_latest_updates(&self, page: u32) -> Result<MangaPage>;
    async fn search_manga(&self, query: &str, page: u32, filters: &[Filter]) -> Result<MangaPage>;
    async fn get_manga_details(&self, manga: &Manga) -> Result<Manga>;
    async fn get_chapter_list(&self, manga: &Manga) -> Result<Vec<Chapter>>;
    async fn get_page_list(&self, chapter: &Chapter) -> Result<Vec<Page>>;
    fn get_filters(&self) -> Vec<Filter>;
}
```

---

#### [NEW] `suwayomi-db` — Database Layer

- **SQLx** with **SQLite** (compile-time verified queries)
- Connection pool via `SqlitePoolOptions`
- Embedded migrations via `sqlx::migrate!()`
- Repository pattern: `MangaRepository`, `ChapterRepository`, `CategoryRepository`, `TrackRepository`

Initial schema (`001_initial.sql`):
```sql
CREATE TABLE IF NOT EXISTS manga (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    artist TEXT,
    author TEXT,
    description TEXT,
    genre TEXT,  -- comma-separated
    status INTEGER NOT NULL DEFAULT 0,
    thumbnail_url TEXT,
    initialized BOOLEAN NOT NULL DEFAULT FALSE,
    in_library BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_id, url)
);

CREATE TABLE IF NOT EXISTS chapter (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    date_upload INTEGER NOT NULL DEFAULT 0,
    chapter_number REAL NOT NULL DEFAULT -1.0,
    scanlator TEXT,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    is_bookmarked BOOLEAN NOT NULL DEFAULT FALSE,
    last_page_read INTEGER NOT NULL DEFAULT 0,
    is_downloaded BOOLEAN NOT NULL DEFAULT FALSE,
    source_order INTEGER NOT NULL DEFAULT 0,
    fetched_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(manga_id, url)
);

CREATE TABLE IF NOT EXISTS category (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS manga_category (
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    PRIMARY KEY (manga_id, category_id)
);

CREATE TABLE IF NOT EXISTS extension (
    pkg_name TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version_name TEXT NOT NULL,
    version_code INTEGER NOT NULL,
    lang TEXT NOT NULL DEFAULT 'en',
    is_nsfw BOOLEAN NOT NULL DEFAULT FALSE,
    is_installed BOOLEAN NOT NULL DEFAULT FALSE,
    has_update BOOLEAN NOT NULL DEFAULT FALSE,
    is_obsolete BOOLEAN NOT NULL DEFAULT FALSE,
    icon_url TEXT,
    apk_url TEXT,
    repo_url TEXT
);

CREATE TABLE IF NOT EXISTS source (
    id TEXT PRIMARY KEY,
    extension_pkg TEXT NOT NULL REFERENCES extension(pkg_name) ON DELETE CASCADE,
    name TEXT NOT NULL,
    lang TEXT NOT NULL DEFAULT 'en',
    icon_url TEXT,
    supports_latest BOOLEAN NOT NULL DEFAULT TRUE,
    is_nsfw BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS track (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    tracker_id TEXT NOT NULL,
    remote_id TEXT,
    title TEXT,
    status INTEGER NOT NULL DEFAULT 0,
    score REAL NOT NULL DEFAULT 0.0,
    total_chapters INTEGER NOT NULL DEFAULT 0,
    last_chapter_read REAL NOT NULL DEFAULT 0.0,
    UNIQUE(manga_id, tracker_id)
);

CREATE TABLE IF NOT EXISTS download_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chapter_id INTEGER NOT NULL REFERENCES chapter(id) ON DELETE CASCADE,
    state INTEGER NOT NULL DEFAULT 0,  -- 0=queued, 1=downloading, 2=done, 3=error
    progress REAL NOT NULL DEFAULT 0.0,
    error TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS server_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_chapter_manga_id ON chapter(manga_id);
CREATE INDEX IF NOT EXISTS idx_manga_category_manga ON manga_category(manga_id);
CREATE INDEX IF NOT EXISTS idx_manga_category_category ON manga_category(category_id);
CREATE INDEX IF NOT EXISTS idx_source_extension ON source(extension_pkg);
CREATE INDEX IF NOT EXISTS idx_track_manga ON track(manga_id);
CREATE INDEX IF NOT EXISTS idx_download_queue_state ON download_queue(state);
```

---

### Phase 2: Extension Runtime & Downloader (Jules Sessions 4-5)

#### [NEW] `suwayomi-extensions` — Extension Loading & Sandbox

**Strategy**: Instead of Dex2Jar (which requires JVM), implement a **JavaScript-based extension runtime** using `boa_engine` (pure Rust JS engine) or `deno_core`/`rquickjs`:

1. **Extension Repository Index**: Fetch extension lists from configurable repos (JSON index)
2. **APK Metadata Parser**: Parse APK manifests to extract extension info (using `zip` crate for APK decompression + minimal AndroidManifest XML binary parser)
3. **JS Extension Bridge**: Provide a JS runtime where extensions implement the `Source` interface via JavaScript. This enables community extensions written in JS (similar to how Mihon extensions could be ported to JS-based scrapers)
4. **Fallback: Wasm Extensions**: Support WASM-compiled extensions via `wasmtime` for near-native performance
5. **HTTP Interceptors**: Extension-scoped HTTP client with cookie jars, custom headers, and CloudFlare bypass (via `reqwest` + WebView fallback)

#### [NEW] `suwayomi-downloader` — Download Queue Engine

- **Priority Queue**: `BinaryHeap` or `tokio::sync::mpsc` channel-based priority queue
- **Worker Pool**: Configurable number of parallel download workers (`max_parallel_downloads`)
- **Pause/Resume/Reorder**: Full queue manipulation via mutations
- **Disk Storage**: `{download_path}/{manga_title}/{chapter_name}/001.jpg` layout
- **Progress Streaming**: Broadcast channel for real-time progress updates to WebSocket subscribers
- **Retry Logic**: Exponential backoff with configurable max retries
- **Bandwidth Limiting**: Optional rate limiter per-source

---

### Phase 3: API Server & WebSocket (Jules Sessions 6-7)

#### [NEW] `suwayomi-api` — GraphQL + REST API

**GraphQL Schema** (async-graphql):
- **Queries**: `manga`, `mangas`, `chapter`, `chapters`, `sources`, `extensions`, `categories`, `downloadQueue`, `serverSettings`
- **Mutations**: `addMangaToLibrary`, `removeMangaFromLibrary`, `updateChapter`, `installExtension`, `uninstallExtension`, `updateExtension`, `enqueueDownload`, `dequeueDownload`, `reorderDownload`, `pauseDownloads`, `resumeDownloads`, `clearDownloads`, `createCategory`, `updateCategory`, `deleteCategory`, `updateServerSettings`
- **Subscriptions**: `downloadChanged`, `extensionChanged`, `mangaChanged`, `libraryChanged`

**REST Endpoints** (for backward compatibility):
```
GET    /api/v1/manga/:id
GET    /api/v1/manga/:id/chapters
GET    /api/v1/manga/:id/thumbnail
GET    /api/v1/chapter/:id
GET    /api/v1/chapter/:id/page/:index
GET    /api/v1/source/list
GET    /api/v1/source/:id/popular/:page
GET    /api/v1/source/:id/latest/:page
GET    /api/v1/source/:id/search
GET    /api/v1/extension/list
POST   /api/v1/extension/install
POST   /api/v1/extension/update
DELETE /api/v1/extension/uninstall
GET    /api/v1/category
GET    /api/v1/downloads
POST   /api/v1/downloads/start
POST   /api/v1/downloads/stop
POST   /api/v1/downloads/clear
GET    /api/v1/settings
```

**Middleware Stack**:
- `tower-http::CompressionLayer` (gzip/brotli)
- `tower-http::CorsLayer` (configurable origins)
- `tower-http::TraceLayer` (structured request logging)
- Optional Basic Auth
- Optional rate limiting (`governor` crate)
- Static file serving for WebUI assets

---

### Phase 4: Binary, Docker & DevOps (Jules Session 8)

#### [NEW] `suwayomi-server` — Binary Entry Point

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 1. Parse CLI args (clap)
    // 2. Load layered config (default.toml → user config → env vars)
    // 3. Initialize tracing subscriber
    // 4. Run database migrations
    // 5. Initialize extension registry
    // 6. Start download queue workers
    // 7. Build Axum router (GraphQL + REST + WebSocket + static files)
    // 8. Bind and serve
}
```

#### [NEW] Dockerfile — Multi-stage Build

```dockerfile
# Build stage
FROM rust:1.82-alpine AS builder
RUN apk add --no-cache musl-dev sqlite-dev openssl-dev
WORKDIR /build
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl

# Runtime stage
FROM alpine:3.20
RUN apk add --no-cache ca-certificates sqlite-libs
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/suwayomi-server /usr/local/bin/
COPY config/default.toml /etc/suwayomi/default.toml
EXPOSE 4567
VOLUME ["/data"]
ENV SUWAYOMI_DATA_DIR=/data
ENTRYPOINT ["suwayomi-server"]
```

#### [NEW] docker-compose.yml

```yaml
services:
  suwayomi:
    build: .
    ports:
      - "4567:4567"
    volumes:
      - suwayomi-data:/data
    environment:
      - SUWAYOMI_DATA_DIR=/data
      - SUWAYOMI_BIND=0.0.0.0
      - SUWAYOMI_PORT=4567
    restart: unless-stopped

volumes:
  suwayomi-data:
```

#### [NEW] CI/CD Pipeline

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo build --release
```

---

## Jules Dispatch Strategy

We will dispatch **8 parallel Jules sessions** across 4 phases:

| Session | Phase | Task | Crates |
|---|---|---|---|
| 1 | Foundation | Cargo workspace scaffold + config + CLI + README | Root, `suwayomi-server` |
| 2 | Foundation | Domain models, error types, traits | `suwayomi-core` |
| 3 | Foundation | Database schema, migrations, repositories | `suwayomi-db` |
| 4 | Runtime | Extension registry, APK parser, JS runtime | `suwayomi-extensions` |
| 5 | Runtime | Download queue, workers, storage, progress | `suwayomi-downloader` |
| 6 | API | GraphQL schema + REST endpoints + middleware | `suwayomi-api` |
| 7 | API | WebSocket progress + subscription system | `suwayomi-api` (ws) |
| 8 | DevOps | Dockerfile, docker-compose, CI/CD workflows | Root DevOps |

> [!IMPORTANT]
> Sessions 1-3 are dispatched first (Phase 1). Sessions 4-8 depend on the core/db crates and will be dispatched after Phase 1 PRs are merged.

---

## Verification Plan

### Automated Tests
- `cargo test` — unit tests for all crates
- `cargo clippy -- -D warnings` — lint-free
- `cargo fmt --check` — consistent formatting
- `sqlx` compile-time query verification
- Integration tests: start server → GraphQL queries → validate responses

### Manual Verification
- `docker build .` and `docker compose up` — verify container starts in <100ms
- `docker images` — verify image size <30MB
- GraphQL Playground at `http://localhost:4567/graphql` — test queries
- Download queue smoke test with a test extension

---

## Open Questions

> [!NOTE]
> These are design decisions that don't block Phase 1 but will affect later phases:

1. **Extension Format**: Should we target JS-based extensions (easier community adoption, `boa_engine`) or Wasm extensions (better performance, `wasmtime`)? **Recommendation**: Start with JS via `boa_engine` for maximum compatibility with the existing Tachiyomi extension ecosystem, add Wasm as an optional high-performance path later.

2. **WebUI Bundling**: Should we bundle the Suwayomi-WebUI (React) as static assets in the binary, or serve it separately? **Recommendation**: Embed via `rust-embed` for single-binary deployment, with an option to override with an external directory.

3. **Tracker Integration**: MAL/AniList/Kitsu — implement as built-in modules or as extensions? **Recommendation**: Built-in with `reqwest` + OAuth2 flow, matching Suwayomi's existing tracker module approach.
