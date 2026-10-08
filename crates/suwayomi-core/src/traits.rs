use async_trait::async_trait;

use crate::error::Result;
use crate::models::{Chapter, Filter, Manga, MangaPage, Page};

#[async_trait]
pub trait MangaSource: Send + Sync {
    async fn get_popular_manga(&self, page: i32) -> Result<MangaPage>;
    async fn get_latest_updates(&self, page: i32) -> Result<MangaPage>;
    async fn search_manga(&self, query: &str, filters: &[Filter], page: i32) -> Result<MangaPage>;
    async fn get_manga_details(&self, manga: Manga) -> Result<Manga>;
    async fn get_chapter_list(&self, manga: &Manga) -> Result<Vec<Chapter>>;
    async fn get_page_list(&self, chapter: &Chapter) -> Result<Vec<Page>>;
    async fn get_filters(&self) -> Result<Vec<Filter>>;
}
