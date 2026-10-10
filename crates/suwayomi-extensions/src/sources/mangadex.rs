use async_trait::async_trait;
use reqwest::{header, Client};
use serde::Deserialize;
use suwayomi_core::error::Result;
use suwayomi_core::models::{Chapter, Filter, Manga, MangaPage, MangaStatus, Page, PageStatus};
use suwayomi_core::traits::MangaSource;

pub struct MangaDexSource {
    client: Client,
    api_url: String,
}

impl MangaDexSource {
    pub const SOURCE_ID: i64 = 2499283573021221994;
    pub const PKG_NAME: &'static str = "eu.kanade.tachiyomi.extension.all.mangadex";

    pub fn new() -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Self {
            client,
            api_url: "https://api.mangadex.org".to_string(),
        }
    }
}

impl Default for MangaDexSource {
    fn default() -> Self {
        Self::new()
    }
}

impl MangaDexSource {
    pub fn with_api_url(api_url: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("User-Agent", header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap_or_default();

        Self { client, api_url }
    }
}

#[derive(Deserialize)]
struct MangaListResponse {
    data: Vec<MangaData>,
    total: i32,
    offset: i32,
    limit: i32,
}

#[derive(Deserialize)]
struct MangaData {
    id: String,
    attributes: MangaAttributes,
    relationships: Vec<Relationship>,
}

#[derive(Deserialize)]
struct MangaAttributes {
    title: serde_json::Value,
    description: Option<serde_json::Value>,
    status: String,
    tags: Vec<TagData>,
}

#[derive(Deserialize)]
struct TagData {
    attributes: TagAttributes,
}

#[derive(Deserialize)]
struct TagAttributes {
    name: serde_json::Value,
}

#[derive(Deserialize)]
struct Relationship {
    id: String,
    #[serde(rename = "type")]
    rel_type: String,
}

#[derive(Deserialize)]
struct MangaResponse {
    data: MangaData,
}

#[derive(Deserialize)]
struct ChapterListResponse {
    data: Vec<ChapterData>,
}

#[derive(Deserialize)]
struct ChapterData {
    id: String,
    attributes: ChapterAttributes,
}

#[derive(Deserialize)]
struct ChapterAttributes {
    #[allow(dead_code)]
    volume: Option<String>,
    chapter: Option<String>,
    title: Option<String>,
    #[serde(rename = "translatedLanguage")]
    #[allow(dead_code)]
    translated_language: Option<String>,
    #[serde(rename = "publishAt")]
    #[allow(dead_code)]
    publish_at: Option<String>,
}

#[derive(Deserialize)]
struct AtHomeResponse {
    #[serde(rename = "baseUrl")]
    #[allow(dead_code)]
    base_url: String,
    chapter: AtHomeChapter,
}

#[derive(Deserialize)]
struct AtHomeChapter {
    hash: String,
    data: Vec<String>,
}

fn get_title(title_val: &serde_json::Value) -> String {
    if let Some(obj) = title_val.as_object() {
        if let Some(en) = obj.get("en") {
            return en.as_str().unwrap_or("").to_string();
        }
        if let Some((_, val)) = obj.iter().next() {
            return val.as_str().unwrap_or("").to_string();
        }
    }
    "Unknown Title".to_string()
}

fn get_description(desc_val: &Option<serde_json::Value>) -> Option<String> {
    if let Some(val) = desc_val {
        if let Some(obj) = val.as_object() {
            if let Some(en) = obj.get("en") {
                return Some(en.as_str().unwrap_or("").to_string());
            }
            if let Some((_, val)) = obj.iter().next() {
                return Some(val.as_str().unwrap_or("").to_string());
            }
        }
    }
    None
}

fn get_manga_status(status: &str) -> MangaStatus {
    match status {
        "ongoing" => MangaStatus::Ongoing,
        "completed" => MangaStatus::Completed,
        "hiatus" => MangaStatus::OnHiatus,
        "cancelled" => MangaStatus::Cancelled,
        _ => MangaStatus::Unknown,
    }
}

impl MangaDexSource {
    async fn fetch_manga_list(&self, url: &str) -> Result<MangaPage> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| suwayomi_core::error::SuwayomiError::Network(e.to_string()))?;
        let res_data: MangaListResponse = response
            .json()
            .await
            .map_err(|e| suwayomi_core::error::SuwayomiError::Parse(e.to_string()))?;

        let mut manga_list = Vec::new();
        for item in res_data.data {
            let title = get_title(&item.attributes.title);
            let description = get_description(&item.attributes.description);
            let status = get_manga_status(&item.attributes.status);

            let mut cover_art_id = None;
            for rel in item.relationships {
                if rel.rel_type == "cover_art" {
                    cover_art_id = Some(rel.id);
                    break;
                }
            }

            let thumbnail_url = cover_art_id.map(|cover_id| format!("cover:{}", cover_id));

            manga_list.push(Manga {
                id: 0,
                source_id: Self::SOURCE_ID,
                url: format!("/manga/{}", item.id),
                title,
                artist: None,
                author: None,
                description,
                genre: None, // Could parse from tags
                status,
                thumbnail_url,
                update_strategy: 0,
                initialized: false,
            });
        }

        let has_next_page = res_data.offset + res_data.limit < res_data.total;
        Ok(MangaPage {
            manga_list,
            has_next_page,
        })
    }
}

#[async_trait]
impl MangaSource for MangaDexSource {
    async fn get_popular_manga(&self, page: i32) -> Result<MangaPage> {
        let offset = (page - 1) * 20;
        let url = format!(
            "{}/manga?limit=20&offset={}&includes[]=cover_art",
            self.api_url, offset
        );
        self.fetch_manga_list(&url).await
    }

    async fn get_latest_updates(&self, page: i32) -> Result<MangaPage> {
        let offset = (page - 1) * 20;
        let url = format!(
            "{}/manga?limit=20&offset={}&includes[]=cover_art&order[updatedAt]=desc",
            self.api_url, offset
        );
        self.fetch_manga_list(&url).await
    }

    async fn search_manga(&self, query: &str, _filters: &[Filter], page: i32) -> Result<MangaPage> {
        let offset = (page - 1) * 20;
        let url = format!(
            "{}/manga?limit=20&offset={}&includes[]=cover_art&title={}",
            self.api_url,
            offset,
            urlencoding::encode(query)
        );
        self.fetch_manga_list(&url).await
    }

    async fn get_manga_details(&self, mut manga: Manga) -> Result<Manga> {
        let id = manga.url.replace("/manga/", "");
        let url = format!(
            "{}/manga/{}?includes[]=author&includes[]=artist&includes[]=cover_art",
            self.api_url, id
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| suwayomi_core::error::SuwayomiError::Network(e.to_string()))?;
        let res_data: MangaResponse = response
            .json()
            .await
            .map_err(|e| suwayomi_core::error::SuwayomiError::Parse(e.to_string()))?;

        manga.title = get_title(&res_data.data.attributes.title);
        manga.description = get_description(&res_data.data.attributes.description);
        manga.status = get_manga_status(&res_data.data.attributes.status);

        let mut genres = Vec::new();
        for tag in res_data.data.attributes.tags {
            if let Some(obj) = tag.attributes.name.as_object() {
                if let Some(en) = obj.get("en") {
                    if let Some(s) = en.as_str() {
                        genres.push(s.to_string());
                    }
                }
            }
        }
        if !genres.is_empty() {
            manga.genre = Some(genres);
        }

        let mut authors = Vec::new();
        let mut artists = Vec::new();
        let mut cover_art_file_name = None;

        #[derive(Deserialize)]
        struct RelAttributes {
            name: Option<String>,
            #[serde(rename = "fileName")]
            file_name: Option<String>,
        }

        #[derive(Deserialize)]
        struct RelWithAttr {
            #[serde(rename = "type")]
            rel_type: String,
            attributes: Option<RelAttributes>,
        }

        #[derive(Deserialize)]
        struct MangaResponseWithRel {
            data: MangaDataWithRel,
        }

        #[derive(Deserialize)]
        struct MangaDataWithRel {
            relationships: Vec<RelWithAttr>,
        }

        // Re-parse to get relationship attributes
        let response2 = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| suwayomi_core::error::SuwayomiError::Network(e.to_string()))?;
        if let Ok(res_rel) = response2.json::<MangaResponseWithRel>().await {
            for rel in res_rel.data.relationships {
                if let Some(attr) = rel.attributes {
                    if rel.rel_type == "author" {
                        if let Some(name) = attr.name {
                            authors.push(name);
                        }
                    } else if rel.rel_type == "artist" {
                        if let Some(name) = attr.name.clone() {
                            artists.push(name);
                        }
                    } else if rel.rel_type == "cover_art" {
                        if let Some(file_name) = attr.file_name {
                            cover_art_file_name = Some(file_name);
                        }
                    }
                }
            }
        }

        if !authors.is_empty() {
            manga.author = Some(authors.join(", "));
        }
        if !artists.is_empty() {
            manga.artist = Some(artists.join(", "));
        }
        if let Some(file_name) = cover_art_file_name {
            manga.thumbnail_url = Some(format!(
                "https://uploads.mangadex.org/covers/{}/{}",
                id, file_name
            ));
        }

        manga.initialized = true;
        Ok(manga)
    }

    async fn get_chapter_list(&self, manga: &Manga) -> Result<Vec<Chapter>> {
        let id = manga.url.replace("/manga/", "");
        let mut offset = 0;
        let limit = 500;
        let mut chapters = Vec::new();
        let mut source_order = 0;

        loop {
            let url = format!(
                "{}/manga/{}/feed?limit={}&offset={}&translatedLanguage[]=en&order[chapter]=desc",
                self.api_url, id, limit, offset
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| suwayomi_core::error::SuwayomiError::Network(e.to_string()))?;
            let res_data: ChapterListResponse = response
                .json()
                .await
                .map_err(|e| suwayomi_core::error::SuwayomiError::Parse(e.to_string()))?;

            let count = res_data.data.len();
            if count == 0 {
                break;
            }

            for item in res_data.data {
                let chapter_number = item
                    .attributes
                    .chapter
                    .and_then(|c| c.parse::<f32>().ok())
                    .unwrap_or(0.0);
                let name = if let Some(t) = item.attributes.title {
                    if t.is_empty() {
                        format!("Chapter {}", chapter_number)
                    } else {
                        t
                    }
                } else {
                    format!("Chapter {}", chapter_number)
                };

                chapters.push(Chapter {
                    id: 0,
                    manga_id: manga.id,
                    url: format!("/chapter/{}", item.id),
                    name,
                    date_upload: 0, // Could parse from publishAt
                    chapter_number,
                    scanlator: None, // Could parse from relationships
                    read: false,
                    bookmark: false,
                    last_page_read: 0,
                    date_fetch: 0,
                    source_order,
                });
                source_order += 1;
            }

            offset += limit;
            if count < limit as usize {
                break;
            }
        }

        Ok(chapters)
    }

    async fn get_page_list(&self, chapter: &Chapter) -> Result<Vec<Page>> {
        let id = chapter.url.replace("/chapter/", "");
        let url = format!("{}/at-home/server/{}", self.api_url, id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| suwayomi_core::error::SuwayomiError::Network(e.to_string()))?;
        let res_data: AtHomeResponse = response
            .json()
            .await
            .map_err(|e| suwayomi_core::error::SuwayomiError::Parse(e.to_string()))?;

        let mut pages = Vec::new();
        for (index, file) in res_data.chapter.data.iter().enumerate() {
            let image_url = format!(
                "https://uploads.mangadex.org/data/{}/{}",
                res_data.chapter.hash, file
            );
            pages.push(Page {
                index: index as i32,
                url: chapter.url.clone(),
                image_url: Some(image_url),
                status: PageStatus::Ready,
            });
        }

        Ok(pages)
    }

    async fn get_filters(&self) -> Result<Vec<Filter>> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[tokio::test]
    async fn test_get_popular_manga() {
        let server = MockServer::start();

        let _mock = server.mock(|when, then| {
            when.method(GET)
                .path("/manga")
                .query_param("limit", "20")
                .query_param("offset", "0");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                    "data": [
                        {
                            "id": "123",
                            "attributes": {
                                "title": {"en": "Test Manga"},
                                "description": {"en": "A test manga"},
                                "status": "ongoing",
                                "tags": []
                            },
                            "relationships": []
                        }
                    ],
                    "total": 1,
                    "offset": 0,
                    "limit": 20
                }"#,
                );
        });

        let source = MangaDexSource::with_api_url(server.url(""));
        let result = source.get_popular_manga(1).await;

        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.manga_list.len(), 1);
        assert_eq!(page.manga_list[0].title, "Test Manga");
        assert_eq!(page.manga_list[0].url, "/manga/123");
        assert!(!page.has_next_page);
    }

    #[tokio::test]
    async fn test_search_manga() {
        let server = MockServer::start();

        let _mock = server.mock(|when, then| {
            when.method(GET)
                .path("/manga")
                .query_param("title", "naruto");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                    "data": [
                        {
                            "id": "456",
                            "attributes": {
                                "title": {"en": "Naruto"},
                                "description": null,
                                "status": "completed",
                                "tags": []
                            },
                            "relationships": []
                        }
                    ],
                    "total": 1,
                    "offset": 0,
                    "limit": 20
                }"#,
                );
        });

        let source = MangaDexSource::with_api_url(server.url(""));
        let result = source.search_manga("naruto", &[], 1).await;

        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.manga_list.len(), 1);
        assert_eq!(page.manga_list[0].title, "Naruto");
        assert_eq!(page.manga_list[0].url, "/manga/456");
    }

    #[tokio::test]
    async fn test_get_chapter_list() {
        let server = MockServer::start();

        let _mock = server.mock(|when, then| {
            when.method(GET).path("/manga/123/feed");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                    "data": [
                        {
                            "id": "chap1",
                            "attributes": {
                                "volume": "1",
                                "chapter": "1",
                                "title": "First Chapter",
                                "translatedLanguage": "en"
                            }
                        }
                    ],
                    "total": 1,
                    "offset": 0,
                    "limit": 500
                }"#,
                );
        });

        let source = MangaDexSource::with_api_url(server.url(""));
        let manga = Manga {
            id: 1,
            source_id: MangaDexSource::SOURCE_ID,
            url: "/manga/123".to_string(),
            title: "Test".to_string(),
            artist: None,
            author: None,
            description: None,
            genre: None,
            status: MangaStatus::Ongoing,
            thumbnail_url: None,
            update_strategy: 0,
            initialized: true,
        };

        let result = source.get_chapter_list(&manga).await;

        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        let chapters = result.unwrap();
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].chapter_number, 1.0);
        assert_eq!(chapters[0].url, "/chapter/chap1");
    }

    #[tokio::test]
    async fn test_get_page_list() {
        let server = MockServer::start();

        let _mock = server.mock(|when, then| {
            when.method(GET).path("/at-home/server/chap1");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                    "baseUrl": "https://s2.mangadex.network",
                    "chapter": {
                        "hash": "hash123",
                        "data": ["1.jpg", "2.jpg"]
                    }
                }"#,
                );
        });

        let source = MangaDexSource::with_api_url(server.url(""));
        let chapter = Chapter {
            id: 0,
            manga_id: 1,
            url: "/chapter/chap1".to_string(),
            name: "Chapter 1".to_string(),
            date_upload: 0,
            chapter_number: 1.0,
            scanlator: None,
            read: false,
            bookmark: false,
            last_page_read: 0,
            date_fetch: 0,
            source_order: 0,
        };

        let result = source.get_page_list(&chapter).await;
        if let Err(e) = &result {
            println!("Error: {:?}", e);
        }
        assert!(result.is_ok());
        let pages = result.unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(
            pages[0].image_url.as_ref().unwrap(),
            "https://uploads.mangadex.org/data/hash123/1.jpg"
        );
    }
}
