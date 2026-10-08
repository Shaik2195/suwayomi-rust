use sqlx::{SqlitePool, Row};
use suwayomi_core::models::{Manga, Chapter, Category, MangaStatus};
use suwayomi_core::error::{Result, SuwayomiError};

fn map_manga_status_from_str(status: &str) -> MangaStatus {
    match status {
        "Ongoing" => MangaStatus::Ongoing,
        "Completed" => MangaStatus::Completed,
        "Licensed" => MangaStatus::Licensed,
        "PublishingFinished" => MangaStatus::PublishingFinished,
        "Cancelled" => MangaStatus::Cancelled,
        "OnHiatus" => MangaStatus::OnHiatus,
        _ => MangaStatus::Unknown,
    }
}

fn map_manga_status_to_str(status: &MangaStatus) -> &'static str {
    match status {
        MangaStatus::Unknown => "Unknown",
        MangaStatus::Ongoing => "Ongoing",
        MangaStatus::Completed => "Completed",
        MangaStatus::Licensed => "Licensed",
        MangaStatus::PublishingFinished => "PublishingFinished",
        MangaStatus::Cancelled => "Cancelled",
        MangaStatus::OnHiatus => "OnHiatus",
    }
}

// Manga Repository
pub struct MangaRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> MangaRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, manga: &Manga) -> Result<i64> {
        let genre_json = match &manga.genre {
            Some(g) => Some(serde_json::to_string(g).map_err(|e| SuwayomiError::Parse(e.to_string()))?),
            None => None,
        };
        let status_str = map_manga_status_to_str(&manga.status);
        
        let result = sqlx::query(
            r#"
            INSERT INTO manga (source_id, url, title, artist, author, description, genre, status, thumbnail_url, update_strategy, initialized)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(manga.source_id)
        .bind(&manga.url)
        .bind(&manga.title)
        .bind(&manga.artist)
        .bind(&manga.author)
        .bind(&manga.description)
        .bind(genre_json)
        .bind(status_str)
        .bind(&manga.thumbnail_url)
        .bind(manga.update_strategy)
        .bind(manga.initialized)
        .execute(self.pool)
        .await
        .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Manga>> {
        let row = sqlx::query("SELECT * FROM manga WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        if let Some(r) = row {
            Ok(Some(self.map_row_to_manga(&r)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_by_url(&self, url: &str) -> Result<Option<Manga>> {
        let row = sqlx::query("SELECT * FROM manga WHERE url = ?")
            .bind(url)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        if let Some(r) = row {
            Ok(Some(self.map_row_to_manga(&r)))
        } else {
            Ok(None)
        }
    }

    pub async fn get_library(&self) -> Result<Vec<Manga>> {
        let rows = sqlx::query("SELECT * FROM manga WHERE initialized = 1")
            .fetch_all(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| self.map_row_to_manga(r)).collect())
    }

    pub async fn update(&self, manga: &Manga) -> Result<()> {
        let genre_json = match &manga.genre {
            Some(g) => Some(serde_json::to_string(g).map_err(|e| SuwayomiError::Parse(e.to_string()))?),
            None => None,
        };
        let status_str = map_manga_status_to_str(&manga.status);

        sqlx::query(
            r#"
            UPDATE manga
            SET source_id = ?, url = ?, title = ?, artist = ?, author = ?, description = ?, genre = ?, status = ?, thumbnail_url = ?, update_strategy = ?, initialized = ?
            WHERE id = ?
            "#
        )
        .bind(manga.source_id)
        .bind(&manga.url)
        .bind(&manga.title)
        .bind(&manga.artist)
        .bind(&manga.author)
        .bind(&manga.description)
        .bind(genre_json)
        .bind(status_str)
        .bind(&manga.thumbnail_url)
        .bind(manga.update_strategy)
        .bind(manga.initialized)
        .bind(manga.id)
        .execute(self.pool)
        .await
        .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM manga WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;
        Ok(())
    }

    fn map_row_to_manga(&self, row: &sqlx::sqlite::SqliteRow) -> Manga {
        let genre_str: Option<String> = row.get("genre");
        let genre: Option<Vec<String>> = genre_str.and_then(|g| serde_json::from_str(&g).ok());
        let status_str: String = row.get("status");

        Manga {
            id: row.get("id"),
            source_id: row.get("source_id"),
            url: row.get("url"),
            title: row.get("title"),
            artist: row.get("artist"),
            author: row.get("author"),
            description: row.get("description"),
            genre,
            status: map_manga_status_from_str(&status_str),
            thumbnail_url: row.get("thumbnail_url"),
            update_strategy: row.get("update_strategy"),
            initialized: row.get("initialized"),
        }
    }
}

// Chapter Repository
pub struct ChapterRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ChapterRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_chapters(&self, chapters: &[Chapter]) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(|e| SuwayomiError::Database(e.to_string()))?;
        
        for chapter in chapters {
            sqlx::query(
                r#"
                INSERT INTO chapter (manga_id, url, name, date_upload, chapter_number, scanlator, read, bookmark, last_page_read, date_fetch, source_order)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(chapter.manga_id)
            .bind(&chapter.url)
            .bind(&chapter.name)
            .bind(chapter.date_upload)
            .bind(chapter.chapter_number)
            .bind(&chapter.scanlator)
            .bind(chapter.read)
            .bind(chapter.bookmark)
            .bind(chapter.last_page_read)
            .bind(chapter.date_fetch)
            .bind(chapter.source_order)
            .execute(&mut *tx)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;
        }
        
        tx.commit().await.map_err(|e| SuwayomiError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_by_manga_id(&self, manga_id: i64) -> Result<Vec<Chapter>> {
        let rows = sqlx::query("SELECT * FROM chapter WHERE manga_id = ? ORDER BY source_order ASC")
            .bind(manga_id)
            .fetch_all(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| self.map_row_to_chapter(r)).collect())
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Chapter>> {
        let row = sqlx::query("SELECT * FROM chapter WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        if let Some(r) = row {
            Ok(Some(self.map_row_to_chapter(&r)))
        } else {
            Ok(None)
        }
    }

    pub async fn update_read(&self, id: i64, read: bool, last_page_read: i64) -> Result<()> {
        sqlx::query("UPDATE chapter SET read = ?, last_page_read = ? WHERE id = ?")
            .bind(read)
            .bind(last_page_read)
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;
        Ok(())
    }

    fn map_row_to_chapter(&self, row: &sqlx::sqlite::SqliteRow) -> Chapter {
        Chapter {
            id: row.get("id"),
            manga_id: row.get("manga_id"),
            url: row.get("url"),
            name: row.get("name"),
            date_upload: row.get("date_upload"),
            chapter_number: row.get("chapter_number"),
            scanlator: row.get("scanlator"),
            read: row.get("read"),
            bookmark: row.get("bookmark"),
            last_page_read: row.get("last_page_read"),
            date_fetch: row.get("date_fetch"),
            source_order: row.get("source_order"),
        }
    }
}

// Category Repository
pub struct CategoryRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> CategoryRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_categories(&self) -> Result<Vec<Category>> {
        let rows = sqlx::query("SELECT * FROM category ORDER BY `order` ASC")
            .fetch_all(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        Ok(rows.iter().map(|r| self.map_row_to_category(r)).collect())
    }

    pub async fn insert_category(&self, category: &Category) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO category (name, `order`, flags)
            VALUES (?, ?, ?)
            "#
        )
        .bind(&category.name)
        .bind(category.order)
        .bind(category.flags)
        .execute(self.pool)
        .await
        .map_err(|e| SuwayomiError::Database(e.to_string()))?;

        Ok(result.last_insert_rowid())
    }

    pub async fn delete_category(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM category WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await
            .map_err(|e| SuwayomiError::Database(e.to_string()))?;
        Ok(())
    }

    fn map_row_to_category(&self, row: &sqlx::sqlite::SqliteRow) -> Category {
        Category {
            id: row.get("id"),
            name: row.get("name"),
            order: row.get("order"),
            flags: row.get("flags"),
        }
    }
}
