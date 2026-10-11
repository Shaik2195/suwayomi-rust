# Suwayomi-Rust: Project Context & Architecture Memory

## Project Overview
Suwayomi-Rust is a high-performance Rust rewrite of [Suwayomi-Server](https://github.com/Suwayomi/Suwayomi-Server), achieving architectural and functional parity with the original JVM server and bundled Tachidesk-WebUI frontend.

- **Repository**: `Shaik2195/suwayomi-rust`
- **Branch**: `main`
- **Docker Container**: `suwayomi` running `suwayomi-rust:latest` (`--network host`, volume `suwayomi_data:/data`)
- **Default Port**: `http://127.0.0.1:4567`
- **Reference Server**: `http://casaos.local:4567`

---

## Key Subsystems & Architecture

1. **`suwayomi-core`**:
   - Domain models: `Manga`, `Chapter`, `Category`, `Page`, `Extension`, `Source`.
   - Traits: `MangaSource` with asynchronous methods (`get_popular_manga`, `get_latest_updates`, `search_manga`, `get_manga_details`, `get_chapter_list`, `get_page_list`, `get_filters`).

2. **`suwayomi-db`**:
   - SQLite with `sqlx`, connection pool, and migrations (`migrations/`).
   - Repositories: `MangaRepository`, `ChapterRepository`, `CategoryRepository`.

3. **`suwayomi-extensions`**:
   - Sandboxed JavaScript runtime with Boa.
   - Keiyoushi Protobuf index decoder (`index.pb`) parsing 1,400+ community extensions.
   - APK downloader and storage in `/data/extensions`.
   - **Native Source Implementations**:
     - `AllMangaSource` (ID `8861274191478178487`, package `eu.kanade.tachiyomi.extension.en.allanime`).
     - `MangaDexSource` (ID `2499283573021221994`, package `eu.kanade.tachiyomi.extension.all.mangadex`).
       - Resolves chapter pages directly against authoritative origin `https://uploads.mangadex.org/data/{hash}/{file}` to bypass unreliable community nodes.

4. **`suwayomi-downloader`**:
   - Async download queue and worker pool with concurrency control.
   - Downloads chapter pages directly into storage `/data/downloads`.

5. **`suwayomi-api`**:
   - Built on Axum and async-graphql.
   - GraphQL schema matching Tachidesk v1 spec (`QueryRoot`, `MutationRoot`).
   - REST live image proxy and disk caching engine:
     - `/api/v1/manga/:id/thumbnail`
     - `/api/v1/manga/:manga_id/chapter/:chapter_id/page/:page`
     - Source-aware dynamic headers (`Referer: https://mangadex.org` vs `https://allmanga.to`).
     - Fallback retry mechanism from `*.mangadex.network` to `uploads.mangadex.org`.
   - Embedded static WebUI assets with SPA routing fallback.

6. **Playwright & Testing Rules**:
   - Browser automation strictly via official Playwright MCP server tools (`call_mcp_tool` / `mcp_playwright_*`), never direct CLI script automation.
   - Remote code changes dispatched via Google Jules sessions on repo `Shaik2195/suwayomi-rust`.

7. **Source Browsing & Full WebUI Parity**:
   - `fetchSourceManga` Relay mutation: full support for `POPULAR`, `LATEST`, and `SEARCH` browsing with `hasNextPage` pagination.
   - Automatic database insertion and ID mapping via `MangaRepository` (preserving `in_library` / `initialized` status).
   - Source metadata and configuration mutations (`setSourceMetas`, `deleteSourceMetas`, `setSourceMeta`, `deleteSourceMeta`, `updateSourcePreference`).
   - CasaOS Deliverable: `suwayomi-rust-docker.zip` (containing `suwayomi-rust-docker.tar.gz`, `docker-compose.yml`, and `README.md`).

8. **Empirical Performance Benchmarks (Rust vs. JVM Container)**:
   - **RAM**: Rust uses **15.16 MiB** vs JVM's **687.7 MiB** (**45.3x lower memory / 97.8% reduction**).
   - **CPU**: Rust idle **0.06%** vs JVM's **1.97%** (**32.8x lower CPU**).
   - **Docker Image**: Rust image **193 MB** vs JVM's **2.12 GB** (**11x smaller**).
   - **GraphQL Latency**: Rust averages **0.97ms - 1.41ms** vs JVM's **5.08ms - 12.2ms** (**5.2x to 9.9x faster**).
   - **Concurrency Throughput**: Rust reaches **1,268 req/s** vs JVM's **400 req/s** under load (**3.17x higher throughput**).

9. **Resolved WebUI Integration & Reader Defects**:
   - **Defect 1 (Broken image placeholders)**: Fixed with dynamic referer headers and direct authoritative origin resolution (`uploads.mangadex.org/data/{hash}/{file}`).
   - **Defect 2 (Remove manga from library fails)**: Implemented Relay `updateMangas` mutation correctly unsetting `in_library` and purging non-library records.
   - **Defect 3 & 4 (Download & Mark as read error in library)**: WebUI batch mutation `UPDATE_CHAPTERS` requires `deleteDownloadedChapters`, `deleteDownloadedChapter`, and `trackProgress` mutations, plus `chapter.manga` populated with updated `unreadCount`.
   - **Defect 5 (Migrate button error)**: `MangaType::from_model` populates source metadata (`SourceType`) from `manga.source_id`, preventing `unknown` language fallback.
   - **Defect 6 (Reader tap zone overlay always visible)**: Patched reader config default `tapZone.showPreview = false` permanently in `ReaderService`.
   - **Defect 7 (Reader infinite scroll beyond page 6 & frozen bottom bar)**: Stored and returned real `page_count` in SQLite & GraphQL (instead of 0), unbound placeholder height to prevent 280,000px flex explosion, and made ref callbacks safe for both callback and object refs.
   - **Defect 8 (Track button does nothing)**: Implemented `TrackerNodeList` Relay format and rich `TrackerType` with `isLoggedIn`, `authUrl`, etc.
   - **Defect 9 (Change category does nothing)**: Seeded SQLite with default category (`id: 1, 'Default'`) and migration to satisfy foreign key constraints.

