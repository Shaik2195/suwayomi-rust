use thiserror::Error;

#[derive(Error, Debug)]
pub enum SuwayomiError {
    #[error("Not found: {0}")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_instantiation() {
        let err = SuwayomiError::NotFound("manga with id 1".to_string());
        assert_eq!(err.to_string(), "Not found: manga with id 1");

        let err = SuwayomiError::Database("connection failed".to_string());
        assert_eq!(err.to_string(), "Database error: connection failed");

        let err = SuwayomiError::Extension("invalid format".to_string());
        assert_eq!(err.to_string(), "Extension error: invalid format");

        let err = SuwayomiError::Download("timeout".to_string());
        assert_eq!(err.to_string(), "Download error: timeout");

        let err = SuwayomiError::Network("host unreachable".to_string());
        assert_eq!(err.to_string(), "Network error: host unreachable");

        let err = SuwayomiError::Parse("invalid json".to_string());
        assert_eq!(err.to_string(), "Parse error: invalid json");

        let err = SuwayomiError::Internal("unexpected failure".to_string());
        assert_eq!(err.to_string(), "Internal error: unexpected failure");
    }

    #[test]
    fn test_result_type() {
        fn return_error() -> Result<()> {
            Err(SuwayomiError::Internal("test".to_string()))
        }

        let res = return_error();
        assert!(res.is_err());
        if let Err(SuwayomiError::Internal(msg)) = res {
            assert_eq!(msg, "test");
        } else {
            panic!("Unexpected error type");
        }
    }
}
