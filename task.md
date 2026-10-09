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
- `[ ]` **Session 8** — E2E & UI testing suite for GraphQL & REST endpoints
  - Issue: #9
  - Session ID: `17580404197599752853`
  - URL: https://jules.google.com/session/17580404197599752853
  - Status: In Progress

## Polling
- Poll interval: every 5 minutes via cron
- Tool: Jules MCP only (`jules_list_sessions`)
- No duplicate dispatches

