# Suwayomi-Rust

A complete Rust rewrite of Suwayomi-Server (headless Tachiyomi manga server).

## Architecture

The project is structured as a Cargo workspace with the following crates:

- **suwayomi-core**: Core traits, error definitions, and data models.
- **suwayomi-db**: Database pool, SQLite migrations, and repositories.
- **suwayomi-extensions**: Extension loader, registry, and Boa JS engine runtime for parsing sources.
- **suwayomi-downloader**: Background worker and queue for chapter downloads.
- **suwayomi-api**: GraphQL and REST API layer.
- **suwayomi-server**: The main application runner (CLI).

## Developer Setup

1. Install Rust (latest stable).
2. Install SQLite (`sqlite3`).
3. Clone the repository.
4. Run `cargo build` to build the workspace.
5. Run `cargo run --bin suwayomi-server` to start the server.

## Docker Quickstart

To quickly spin up a test instance using Docker Compose:

```bash
docker-compose up -d --build
```

The server will be available at `http://localhost:4567`.
Data will be persisted in a Docker volume named `suwayomi-data`.
