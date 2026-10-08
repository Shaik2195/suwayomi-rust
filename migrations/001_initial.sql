CREATE TABLE IF NOT EXISTS manga (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    artist TEXT,
    author TEXT,
    description TEXT,
    genre TEXT,
    status INTEGER NOT NULL DEFAULT 0,
    thumbnail_url TEXT,
    source INTEGER NOT NULL,
    initialized BOOLEAN NOT NULL DEFAULT 0,
    in_library BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS chapter (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id INTEGER NOT NULL,
    url TEXT NOT NULL,
    name TEXT NOT NULL,
    scanlator TEXT,
    read BOOLEAN NOT NULL DEFAULT 0,
    bookmark BOOLEAN NOT NULL DEFAULT 0,
    last_page_read INTEGER NOT NULL DEFAULT 0,
    chapter_number REAL NOT NULL DEFAULT -1,
    source_order INTEGER NOT NULL DEFAULT 0,
    date_fetch INTEGER NOT NULL DEFAULT 0,
    date_upload INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS category (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    flags INTEGER NOT NULL DEFAULT 0
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
    is_nsfw BOOLEAN NOT NULL DEFAULT 0,
    has_readme BOOLEAN NOT NULL DEFAULT 0,
    has_changelog BOOLEAN NOT NULL DEFAULT 0,
    is_obsolete BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS source (
    id INTEGER PRIMARY KEY,
    lang TEXT NOT NULL,
    name TEXT NOT NULL,
    supports_latest BOOLEAN NOT NULL DEFAULT 0,
    is_configured BOOLEAN NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS track (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id INTEGER NOT NULL,
    sync_id INTEGER NOT NULL,
    media_id INTEGER NOT NULL,
    library_id INTEGER,
    title TEXT NOT NULL,
    last_chapter_read REAL NOT NULL DEFAULT 0,
    total_chapters INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 0,
    score REAL NOT NULL DEFAULT 0,
    tracking_url TEXT,
    start_date INTEGER NOT NULL DEFAULT 0,
    finish_date INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS download_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    manga_id INTEGER NOT NULL,
    chapter_id INTEGER NOT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(manga_id) REFERENCES manga(id) ON DELETE CASCADE,
    FOREIGN KEY(chapter_id) REFERENCES chapter(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS server_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_manga_in_library ON manga(in_library);
CREATE INDEX idx_manga_source ON manga(source);
CREATE INDEX idx_chapter_manga_id ON chapter(manga_id);
CREATE INDEX idx_manga_category_manga_id ON manga_category(manga_id);
CREATE INDEX idx_manga_category_category_id ON manga_category(category_id);
CREATE INDEX idx_track_manga_id ON track(manga_id);
CREATE INDEX idx_download_queue_manga_id ON download_queue(manga_id);
CREATE INDEX idx_download_queue_chapter_id ON download_queue(chapter_id);
