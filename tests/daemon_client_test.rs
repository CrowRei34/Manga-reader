use async_trait::async_trait;
use bakeneko::core::daemon::api::MangaSourceApi;
use bakeneko::core::error::DaemonError;
use bakeneko::core::models::{Manga, Page, PingReply, Source};
use std::collections::HashMap;

struct MockDaemon;

#[async_trait]
impl MangaSourceApi for MockDaemon {
    async fn ping(&self) -> Result<PingReply, DaemonError> {
        Ok(PingReply { version: "1.0.0".into(), java: "21".into() })
    }
    async fn list_sources(&self) -> Result<Vec<Source>, DaemonError> {
        Ok(vec![Source { id: "MANGADEX".into(), name: "MangaDex".into() }])
    }
    async fn catalog_list(&self, _s: &str, _o: i32, _q: Option<&str>) -> Result<Vec<Manga>, DaemonError> { Ok(vec![]) }
    async fn manga_details(&self, _s: &str, m: &Manga) -> Result<Manga, DaemonError> { Ok(m.clone()) }
    async fn chapter_pages(&self, _s: &str, _c: &bakeneko::core::models::Chapter) -> Result<Vec<Page>, DaemonError> { Ok(vec![]) }
    async fn page_url(&self, _s: &str, _p: &Page) -> Result<String, DaemonError> { Ok("http://x/1.jpg".into()) }
    async fn source_headers(&self, _s: &str) -> Result<HashMap<String, String>, DaemonError> { Ok(HashMap::new()) }
}

#[tokio::test]
async fn mock_satisfies_api() {
    let m = MockDaemon;
    let p = m.ping().await.unwrap();
    assert_eq!(p.version, "1.0.0");
    let srcs = m.list_sources().await.unwrap();
    assert_eq!(srcs[0].id, "MANGADEX");
}
