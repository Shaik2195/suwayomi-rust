use crate::types::ExtensionListing;
use std::io::{Cursor, Read};
use suwayomi_core::error::{Result, SuwayomiError};
use zip::ZipArchive;

pub struct ExtensionLoader;

impl ExtensionLoader {
    pub async fn download_extension(listing: &ExtensionListing) -> Result<Vec<u8>> {
        let response = reqwest::get(&listing.apk_url)
            .await
            .map_err(|e| SuwayomiError::Network(e.to_string()))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| SuwayomiError::Network(e.to_string()))?;

        Ok(bytes.to_vec())
    }

    pub fn inspect_archive(apk_bytes: &[u8]) -> Result<Vec<String>> {
        let cursor = Cursor::new(apk_bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to open zip archive: {}", e)))?;

        let mut files = Vec::new();
        for i in 0..archive.len() {
            let file = archive.by_index(i).map_err(|e| {
                SuwayomiError::Extension(format!("Failed to read file in archive: {}", e))
            })?;
            files.push(file.name().to_string());
        }

        Ok(files)
    }

    pub fn extract_file(apk_bytes: &[u8], file_path: &str) -> Result<Vec<u8>> {
        let cursor = Cursor::new(apk_bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to open zip archive: {}", e)))?;

        let mut file = archive.by_name(file_path).map_err(|e| {
            SuwayomiError::Extension(format!(
                "Failed to find file {} in archive: {}",
                file_path, e
            ))
        })?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|e| SuwayomiError::Extension(format!("Failed to read file content: {}", e)))?;

        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::{FileOptions, ZipWriter};

    #[test]
    fn test_inspect_and_extract_archive() {
        let mut buf = Vec::new();
        {
            let cursor = Cursor::new(&mut buf);
            let mut zip = ZipWriter::new(cursor);
            let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

            zip.start_file("index.js", options).unwrap();
            zip.write_all(b"console.log('test');").unwrap();

            zip.start_file("package.json", options).unwrap();
            zip.write_all(b"{\"name\": \"test\"}").unwrap();

            zip.finish().unwrap();
        }

        let files = ExtensionLoader::inspect_archive(&buf).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"index.js".to_string()));
        assert!(files.contains(&"package.json".to_string()));

        let content = ExtensionLoader::extract_file(&buf, "index.js").unwrap();
        assert_eq!(content, b"console.log('test');");
    }
}
