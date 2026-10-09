# Suwayomi Rust Rewrite — Task Tracker

## Dispatch Strategy (Conflict-Free)

Each session only touches files within its own crate directory. Session 1 creates the entire skeleton first so no subsequent session needs to modify shared files like root `Cargo.toml`.

## Phase 1: Scaffold
- `[x]` **Session 1** — Workspace scaffold (ALL Cargo.tomls, stub files, config, Dockerfile, CI, README)
  - Files: `Cargo.toml`, `crates/*/Cargo.toml`, `crates/*/src/lib.rs`, `config/`, `migrations/`, `Dockerfile`, `docker-compose.yml`, `.github/`, `README.md`
  - Session ID: `8391700677865273778`
  - URL: https://jules.google.com/session/8391700677865273778
  - Status: Completed (applied & pushed to `main`)

## Phase 2: Core + DB
- `[x]` **Session 2** — `suwayomi-core`: domain models, error types, traits
  - Files: `crates/suwayomi-core/src/**`
  - Session ID: `1812315603635678826`
  - URL: https://jules.google.com/session/1812315603635678826
  - Status: Completed (applied & pushed to `main`)
- `[x]` **Session 3** — `suwayomi-db`: SQLx repos, migrations, pool
  - Files: `crates/suwayomi-db/src/**`, `migrations/*.sql`
  - Session ID: `10240266895646774019`
  - URL: https://jules.google.com/session/10240266895646774019
  - Status: Completed (applied & pushed to `main`)

## Phase 3: Extensions + Downloader (Parallel)
- `[x]` **Session 4** — `suwayomi-extensions`: registry, loader, JS runtime
  - Files: `crates/suwayomi-extensions/src/**`
  - Session ID: `676689372886668376`
  - URL: https://jules.google.com/session/676689372886668376
  - Status: Completed (applied & pushed to `main`)
- `[x]` **Session 5** — `suwayomi-downloader`: queue, workers, storage
  - Files: `crates/suwayomi-downloader/src/**`
  - Session ID: `9448793703678203263`
  - URL: https://jules.google.com/session/9448793703678203263
  - Status: Completed (applied & pushed to `main`)

## Phase 4: API (Solo)
- `[x]` **Session 6** — `suwayomi-api`: GraphQL + REST + WebSocket + middleware
  - Files: `crates/suwayomi-api/src/**`
  - Session ID: `5459145112336095199`
  - URL: https://jules.google.com/session/5459145112336095199
  - Status: Completed (applied & pushed to `main`)

## Phase 5: Server Binary (Solo)
- `[x]` **Session 7** — `suwayomi-server`: main.rs wiring, integration, Docker verify
  - Files: `crates/suwayomi-server/src/**`
  - Session ID: `280927253613283980`
  - URL: https://jules.google.com/session/280927253613283980
  - Status: Completed (applied & pushed to `main`)

## Phase 6: E2E & UI Testing (Solo)
- `[x]` **Session 8** — E2E & UI testing suite for GraphQL & REST endpoints
  - Issue: #9 (closed)
  - Session ID: `17580404197599752853`
  - URL: https://jules.google.com/session/17580404197599752853
  - Status: Completed (applied & pushed to `main`)
  - Local Playwright Verification: Tested live via Playwright MCP (`browser_navigate`, `browser_snapshot`, `browser_take_screenshot`). Verified GraphQL Playground UI, interactive query execution, schema introspection, and REST `/api/v1/category`.

## Phase 7: Architectural Review & Gap Analysis (Solo)
- `[x]` **Session 9** — Architectural review of Suwayomi-Server + Tachidesk-WebUI vs suwayomi-rust
  - Issue: #10 (closed)
  - Session ID: `10033699167618502008`
  - URL: https://jules.google.com/session/10033699167618502008
  - Status: Completed (docs/ARCHITECTURE_REVIEW.md created)
  - Deliverable: Comprehensive subsystem review, Mermaid diagram, gap matrix, and 4-phase technical roadmap.

## Phase 8: API & WebUI Contract Parity (Solo)
- `[x]` **Session 10** — Tachidesk-WebUI contract parity, dual endpoints, camelCase GraphQL, image routes
  - Issue: #11 (closed)
  - Session ID: `6978530845011705030`
  - URL: https://jules.google.com/session/6978530845011705030
  - Status: Completed (applied & pushed to `main`)
  - Deliverables: `/api/graphql` alias, camelCase GraphQL mappings (`isRead`, `thumbnailUrl`, `pageCount`, etc.), root queries (`aboutServer`, `aboutWebUI`, `settings`, `metas`, `downloadStatus`), and `/thumbnail` & `/page/:page` image REST routes.

## Phase 9: Downloader Event Streaming & WebSocket Feeds (Solo)
- `[x]` **Session 11** — Downloader event broadcast, live WebSocket feeds, queue control
  - Issue: #12 (closed)
  - Session ID: `1188902396460985093`
  - URL: https://jules.google.com/session/1188902396460985093
  - Status: Completed (applied & pushed to `main`)
  - Deliverables: `DownloadEvent` enum, `broadcast` channels in `DownloadQueue`, real-time JSON streaming over `/ws`, download mutations in GraphQL, and worker pool initialization in `main.rs`.







