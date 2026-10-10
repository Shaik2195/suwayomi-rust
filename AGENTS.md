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
