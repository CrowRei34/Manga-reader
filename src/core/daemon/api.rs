use crate::core::error::DaemonError;
use crate::core::models::{Manga, Page, PingReply, Source};
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait MangaSourceApi: Send + Sync {
    async fn ping(&self) -> Result<PingReply, DaemonError>;
    async fn list_sources(&self) -> Result<Vec<Source>, DaemonError>;
    async fn catalog_list(&self, source: &str, offset: i32, query: Option<&str>) -> Result<Vec<Manga>, DaemonError>;
    async fn manga_details(&self, source: &str, manga: &Manga) -> Result<Manga, DaemonError>;
    async fn chapter_pages(&self, source: &str, chapter: &crate::core::models::Chapter) -> Result<Vec<Page>, DaemonError>;
    async fn page_url(&self, source: &str, page: &Page) -> Result<String, DaemonError>;
    async fn source_headers(&self, source: &str) -> Result<HashMap<String, String>, DaemonError>;
}
