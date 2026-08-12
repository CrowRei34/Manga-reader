// tests/download_manager_test.rs
//
// Gate de Task 17: DownloadManager con cola persistida + eventos.
// Flujo: enqueue (state=Queued) -> poll_once (descarga de 1 página
// mediante un FakeDaemon + caché pre-poblada) -> state=Done.

use async_trait::async_trait;
use bakeneko::core::daemon::api::MangaSourceApi;
use bakeneko::core::db::Database;
use bakeneko::core::db::dao::{download_dao, manga_dao};
use bakeneko::core::downloads::{DownloadEvent, DownloadManager};
use bakeneko::core::error::DaemonError;
use bakeneko::core::models::{Chapter, DownloadState, Manga, Page, PingReply, Source};
use bakeneko::core::net::ImageCache;
use std::collections::HashMap;
use std::sync::Arc;

const PAGE_URL: &str = "http://127.0.0.1/1.jpg";

struct FakeDaemon;

#[async_trait]
impl MangaSourceApi for FakeDaemon {
    async fn ping(&self) -> Result<PingReply, DaemonError> {
        Ok(PingReply { version: "1".into(), java: "21".into() })
    }
    async fn list_sources(&self) -> Result<Vec<Source>, DaemonError> {
        Ok(vec![])
    }
    async fn catalog_list(
        &self,
        _s: &str,
        _o: i32,
        _q: Option<&str>,
    ) -> Result<Vec<Manga>, DaemonError> {
        Ok(vec![])
    }
    async fn manga_details(&self, _s: &str, m: &Manga) -> Result<Manga, DaemonError> {
        Ok(m.clone())
    }
    async fn chapter_pages(
        &self,
        _s: &str,
        _c: &Chapter,
    ) -> Result<Vec<Page>, DaemonError> {
        Ok(vec![Page {
            source: "S".into(),
            url: PAGE_URL.to_string(),
            preview: None,
        }])
    }
    async fn page_url(&self, _s: &str, _p: &Page) -> Result<String, DaemonError> {
        Ok(PAGE_URL.to_string())
    }
    async fn source_headers(
        &self,
        _s: &str,
    ) -> Result<HashMap<String, String>, DaemonError> {
        Ok(HashMap::new())
    }
}

fn sample_manga() -> Manga {
    Manga {
        source: "MANGADEX".into(),
        url: "/m1".into(),
        title: "T".into(),
        ..Default::default()
    }
}

fn sample_chapter() -> Chapter {
    Chapter {
        source: "MANGADEX".into(),
        url: "/c1".into(),
        title: "C1".into(),
        number: 0.0,
        volume: 0,
        scanlator: None,
        upload_date: 0,
        branch: None,
        blob: Default::default(),
        read: false,
    }
}

#[test]
fn enqueue_then_poll_marks_done() {
    // Aislamos el XDG_CACHE_HOME para no tocar el caché real del usuario y
    // para poder pre-poblar el archivo de la página que devolverá el daemon
    // (sin levantar un servidor HTTP real).
    temp_env::with_var("XDG_CACHE_HOME", Some("/tmp/dl-mgr-test-cache"), || {
        let _ = std::fs::remove_dir_all("/tmp/dl-mgr-test-cache");

        let db = Database::open(None).unwrap();
        db.migrate().unwrap();
        let conn = db.connection();

        let cache = Arc::new(ImageCache::new());
        // Pre-poblar la caché: cache.get() encuentra el archivo y devuelve
        // su path sin hacer HTTP contra 127.0.0.1.
        let cached = cache.cached_path(PAGE_URL);
        std::fs::create_dir_all(cached.parent().unwrap()).unwrap();
        std::fs::write(&cached, b"FAKEIMG").unwrap();

        let daemon: Arc<dyn MangaSourceApi> = Arc::new(FakeDaemon);
        let mgr = DownloadManager::new(conn.clone(), daemon, cache, 2);

        let m = sample_manga();
        let _id = manga_dao::upsert(&conn.lock().unwrap(), &m, 0).unwrap();
        let ch = sample_chapter();

        // El receptor debe crearse antes de enqueue para capturar Queued.
        let mut rx = mgr.subscribe();

        mgr.enqueue(&m, &ch).unwrap();
        assert_eq!(
            download_dao::list_by_state(&conn.lock().unwrap(), DownloadState::Queued)
                .unwrap()
                .len(),
            1
        );

        mgr.poll_once().unwrap();

        assert_eq!(
            download_dao::list_by_state(&conn.lock().unwrap(), DownloadState::Done)
                .unwrap()
                .len(),
            1
        );

        // Verificamos la secuencia de eventos: Queued, Progress, Done.
        let e0 = rx.try_recv().unwrap();
        assert!(matches!(e0, DownloadEvent::Queued(id, url) if id > 0 && url == "/c1"));
        let e1 = rx.try_recv().unwrap();
        assert!(matches!(e1, DownloadEvent::Progress { done: 1, total: 1, .. }));
        let e2 = rx.try_recv().unwrap();
        assert!(matches!(e2, DownloadEvent::Done(_, url) if url == "/c1"));
    });
}