use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};
use std::str::FromStr;
use std::time::Duration;
use suwayomi_core::error::{SuwayomiError, Result};

pub async fn create_sqlite_pool(database_url: &str) -> Result<SqlitePool> {
    let connection_options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| SuwayomiError::Database(e.to_string()))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(5000));

    let pool = SqlitePoolOptions::new()
        .connect_with(connection_options)
        .await
        .map_err(|e| SuwayomiError::Database(e.to_string()))?;

    Ok(pool)
}
