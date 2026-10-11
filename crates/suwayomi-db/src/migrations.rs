use sqlx::Executor;
use sqlx::SqlitePool;
use suwayomi_core::error::{Result, SuwayomiError};

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    pool.execute(
        "
        CREATE TABLE IF NOT EXISTS manga (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL,
            url TEXT NOT NULL,
            title TEXT NOT NULL,
            artist TEXT,
            author TEXT,
            description TEXT,
            genre TEXT,
            status TEXT NOT NULL,
            thumbnail_url TEXT,
            update_strategy INTEGER NOT NULL,
            initialized BOOLEAN NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_manga_url ON manga(url);
        CREATE INDEX IF NOT EXISTS idx_manga_source_id ON manga(source_id);

        CREATE TABLE IF NOT EXISTS chapter (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            manga_id INTEGER NOT NULL,
            url TEXT NOT NULL,
            name TEXT NOT NULL,
            date_upload INTEGER NOT NULL,
            chapter_number REAL NOT NULL,
            scanlator TEXT,
            read BOOLEAN NOT NULL,
            bookmark BOOLEAN NOT NULL,
            last_page_read INTEGER NOT NULL,
            date_fetch INTEGER NOT NULL,
            source_order INTEGER NOT NULL,
            page_count INTEGER NOT NULL DEFAULT -1,
            FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_chapter_manga_id ON chapter(manga_id);

        CREATE TABLE IF NOT EXISTS category (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            `order` INTEGER NOT NULL,
            flags INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS manga_category (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            manga_id INTEGER NOT NULL,
            category_id INTEGER NOT NULL,
            FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE,
            FOREIGN KEY(category_id) REFERENCES category(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS extension (
            pkg_name TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version_name TEXT NOT NULL,
            version_code INTEGER NOT NULL,
            lang TEXT NOT NULL,
            is_nsfw BOOLEAN NOT NULL,
            has_readme BOOLEAN NOT NULL,
            has_changelog BOOLEAN NOT NULL,
            is_obsolete BOOLEAN NOT NULL,
            is_unofficial BOOLEAN NOT NULL
        );

        CREATE TABLE IF NOT EXISTS source (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            lang TEXT NOT NULL,
            supports_latest BOOLEAN NOT NULL,
            is_configured BOOLEAN NOT NULL
        );

        CREATE TABLE IF NOT EXISTS track (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            manga_id INTEGER NOT NULL,
            sync_id INTEGER NOT NULL,
            remote_id INTEGER NOT NULL,
            library_id INTEGER,
            title TEXT NOT NULL,
            last_chapter_read REAL NOT NULL,
            total_chapters INTEGER NOT NULL,
            status INTEGER NOT NULL,
            score REAL NOT NULL,
            tracking_url TEXT NOT NULL,
            start_date INTEGER NOT NULL,
            finish_date INTEGER NOT NULL,
            FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS download_queue (
            chapter_id INTEGER PRIMARY KEY,
            manga_id INTEGER NOT NULL,
            status TEXT NOT NULL,
            FOREIGN KEY(chapter_id) REFERENCES chapter(id) ON DELETE CASCADE,
            FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS server_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS manga_meta (
            manga_id INTEGER NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            PRIMARY KEY (manga_id, key),
            FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_manga_meta_manga_id ON manga_meta(manga_id);

        CREATE TABLE IF NOT EXISTS source_meta (
            source_id INTEGER NOT NULL,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            PRIMARY KEY (source_id, key)
        );
        CREATE INDEX IF NOT EXISTS idx_source_meta_source_id ON source_meta(source_id);
        ",
    )
    .await
    .map_err(|e| SuwayomiError::Database(e.to_string()))?;

    // Migrate existing tables: add page_count to chapter if not present
    let _ = sqlx::query("ALTER TABLE chapter ADD COLUMN page_count INTEGER NOT NULL DEFAULT -1")
        .execute(pool)
        .await;

    // Ensure default category (id 1, 'Default') exists for category assignment
    let _ = sqlx::query("INSERT OR IGNORE INTO category (id, name, `order`, flags) VALUES (1, 'Default', 0, 0)")
        .execute(pool)
        .await;

    Ok(())
}
