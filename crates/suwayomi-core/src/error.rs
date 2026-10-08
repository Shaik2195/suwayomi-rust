use thiserror::Error;

#[derive(Error, Debug)]
pub enum SuwayomiError {
    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Extension error: {0}")]
    Extension(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SuwayomiError>;
