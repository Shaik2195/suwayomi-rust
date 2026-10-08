use crate::types::{ExtensionListing, ExtensionRepo};
use suwayomi_core::error::{Result, SuwayomiError};
use reqwest;

pub struct ExtensionRegistry {
    repos: Vec<ExtensionRepo>,
}

impl ExtensionRegistry {
    pub fn new(repos: Vec<ExtensionRepo>) -> Self {
        Self { repos }
    }

    pub async fn fetch_repo_index(repo: &ExtensionRepo) -> Result<Vec<ExtensionListing>> {
        let response = reqwest::get(&repo.url)
            .await
            .map_err(|e| SuwayomiError::Network(e.to_string()))?;
            
        let listings = response
            .json::<Vec<ExtensionListing>>()
            .await
            .map_err(|e| SuwayomiError::Parse(e.to_string()))?;
            
        Ok(listings)
    }

    pub async fn get_available_extensions(&self) -> Result<Vec<ExtensionListing>> {
        let mut all_listings = Vec::new();
        
        for repo in &self.repos {
            match Self::fetch_repo_index(repo).await {
                Ok(listings) => all_listings.extend(listings),
                Err(e) => {
                    // Log error but continue with other repos
                    eprintln!("Failed to fetch repo index from {}: {}", repo.url, e);
                }
            }
        }
        
        Ok(all_listings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn test_fetch_repo_index() {
        let server = MockServer::start();
        
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/index.min.json");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"[
                    {
                        "pkg_name": "eu.kanade.tachiyomi.extension.en.mangadex",
                        "name": "MangaDex",
                        "version_name": "1.2.3",
                        "version_code": 12,
                        "lang": "en",
                        "is_nsfw": false,
                        "apk_url": "/apk/mangadex.apk",
                        "icon_url": "/icon/mangadex.png"
                    }
                ]"#);
        });
        
        let repo = ExtensionRepo {
            name: "Test Repo".to_string(),
            url: server.url("/index.min.json"),
        };
        
        let listings = ExtensionRegistry::fetch_repo_index(&repo).await;
        
        assert!(listings.is_ok());
        let listings = listings.unwrap();
        
        assert_eq!(listings.len(), 1);
        assert_eq!(listings[0].name, "MangaDex");
        assert_eq!(listings[0].version_code, 12);
    }
}
