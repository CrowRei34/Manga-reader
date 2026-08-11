// Repro en vivo del flujo completo del lector contra el daemon real:
// catalog.list → manga.details → chapter.pages → page.url → GET imagen.
// Cada paso con timeout para localizar dónde se cuelga el AppImage.
// Correr con: cargo test --test reader_flow_live -- --nocapture --ignored
use bakeneko::core::daemon::api::MangaSourceApi;
use bakeneko::core::daemon::client::DaemonClient;
use std::time::Duration;

async fn step<T, F: std::future::Future<Output = Result<T, bakeneko::core::error::DaemonError>>>(
    name: &str,
    secs: u64,
    fut: F,
) -> Option<T> {
    match tokio::time::timeout(Duration::from_secs(secs), fut).await {
        Err(_) => {
            eprintln!("✗ {name}: TIMEOUT tras {secs}s  ← AQUÍ SE CUELGA");
            None
        }
        Ok(Err(e)) => {
            eprintln!("✗ {name}: ERROR: {e}");
            None
        }
        Ok(Ok(v)) => {
            eprintln!("✓ {name}");
            Some(v)
        }
    }
}

#[tokio::test]
#[ignore]
async fn reader_flow() {
    let jar = DaemonClient::default_jar_path();
    let client = DaemonClient::new();
    client
        .start(Some(jar.to_str().unwrap()), None)
        .await
        .expect("daemon start");

    let ping = step("ping", 20, client.ping()).await.expect("ping");
    eprintln!("  daemon v{} java {}", ping.version, ping.java);

    let sources = step("sources.list", 20, client.list_sources()).await.expect("sources");
    eprintln!("  {} fuentes", sources.len());

    // La fuente a probar (por defecto MANGADEX; override con SOURCE=...).
    let source = std::env::var("SOURCE").unwrap_or_else(|_| "MANGADEX".into());

    let list = step(
        &format!("catalog.list {source}"),
        30,
        client.catalog_list(&source, 0, None),
    )
    .await
    .expect("catalog");
    eprintln!("  {} mangas", list.len());
    let manga = list.first().expect("catálogo vacío").clone();
    eprintln!("  manga[0] = {}", manga.title);

    let full = step("manga.details", 30, client.manga_details(&source, &manga))
        .await
        .expect("details");
    eprintln!("  {} capítulos", full.chapters.len());
    let chapter = full.chapters.first().expect("sin capítulos").clone();
    eprintln!("  cap[0] = {} ({})", chapter.title, chapter.url);

    let pages = step("chapter.pages", 45, client.chapter_pages(&source, &chapter))
        .await
        .expect("pages");
    eprintln!("  {} páginas", pages.len());
    let page = pages.first().expect("sin páginas").clone();

    let url = step("page.url", 45, client.page_url(&source, &page))
        .await
        .expect("page_url");
    eprintln!("  page_url = {url}");

    let headers = step("source.headers", 20, client.source_headers(&source))
        .await
        .unwrap_or_default();
    eprintln!("  headers = {headers:?}");

    // Descarga real de la imagen como hace ImageCache.
    let cache = bakeneko::core::net::ImageCache::new();
    match tokio::time::timeout(Duration::from_secs(45), cache.get(&url, &headers)).await {
        Err(_) => eprintln!("✗ imagen: TIMEOUT tras 45s  ← AQUÍ SE CUELGA"),
        Ok(Err(e)) => eprintln!("✗ imagen: ERROR: {e}"),
        Ok(Ok(p)) => eprintln!("✓ imagen descargada en {}", p.display()),
    }

    client.stop().await;
}

// Repro de la condición de carrera: N `page.url` concurrentes (como hace el
// reader con Task::batch) sobre el mismo socket.
#[tokio::test]
#[ignore]
async fn concurrent_page_urls() {
    let jar = DaemonClient::default_jar_path();
    let client = std::sync::Arc::new(DaemonClient::new());
    client.start(Some(jar.to_str().unwrap()), None).await.expect("daemon start");
    client.ping().await.expect("ping");

    let source = std::env::var("SOURCE").unwrap_or_else(|_| "MANGADEX".into());
    let list = client.catalog_list(&source, 0, None).await.expect("catalog");

    // Busca un capítulo con varias páginas (así reproduce el batch real);
    // algunos mangas fallan en details (feeds enormes) — se saltan.
    let mut pages = Vec::new();
    'outer: for manga in list.iter().take(8) {
        let Ok(full) = client.manga_details(&source, manga).await else { continue };
        for ch in full.chapters.iter().take(10) {
            let Ok(p) = client.chapter_pages(&source, ch).await else { continue };
            eprintln!("'{}' cap '{}' → {} páginas", manga.title, ch.title, p.len());
            if p.len() >= 8 {
                pages = p;
                break 'outer;
            }
        }
    }
    assert!(pages.len() >= 8, "no encontré capítulo con ≥8 páginas");

    eprintln!("Disparando {} page.url CONCURRENTES…", pages.len());
    let futs = pages.iter().map(|p| {
        let c = client.clone();
        let p = p.clone();
        async move { c.page_url(&p.source, &p).await }
    });
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        iced::futures::future::join_all(futs),
    )
    .await
    {
        Err(_) => panic!("TIMEOUT: page.url concurrentes se colgaron (carrera de writes confirmada)"),
        Ok(results) => {
            let ok = results.iter().filter(|r| r.is_ok()).count();
            eprintln!("✓ {}/{} page.url resueltos", ok, results.len());
            assert_eq!(ok, results.len(), "algunos page.url fallaron");
        }
    }
    client.stop().await;
}

// Secuencia EXACTA del reader: pages+headers en paralelo → por página
// page.url + descarga de imagen, todo concurrente (como Task::batch).
#[tokio::test]
#[ignore]
async fn exact_reader_batch() {
    let jar = DaemonClient::default_jar_path();
    let client = std::sync::Arc::new(DaemonClient::new());
    client.start(Some(jar.to_str().unwrap()), None).await.expect("daemon start");
    client.ping().await.expect("ping");

    let source = std::env::var("SOURCE").unwrap_or_else(|_| "MANGADEX".into());
    let list = client.catalog_list(&source, 0, None).await.expect("catalog");
    let mut target = None;
    for manga in list.iter().take(8) {
        let Ok(full) = client.manga_details(&source, manga).await else { continue };
        if let Some(ch) = full.chapters.first() {
            target = Some(ch.clone());
            break;
        }
    }
    let chapter = target.expect("sin capítulo");
    eprintln!("cap: {}", chapter.title);

    // Como Message::Load: pages y headers EN PARALELO.
    let (pages, headers) = tokio::join!(
        client.chapter_pages(&source, &chapter),
        client.source_headers(&source),
    );
    let pages = pages.expect("pages");
    let headers = headers.unwrap_or_default();
    eprintln!("{} páginas, headers {:?}", pages.len(), headers);

    // Como PagesFetched(Ok): por página page.url + cache.get, TODO concurrente.
    let cache = std::sync::Arc::new(bakeneko::core::net::ImageCache::new());
    let t0 = std::time::Instant::now();
    let futs = pages.iter().enumerate().map(|(i, p)| {
        let c = client.clone();
        let cache = cache.clone();
        let headers = headers.clone();
        let p = p.clone();
        async move {
            let final_url = c.page_url(&p.source, &p).await.unwrap_or_else(|_| p.url.clone());
            let path = cache.get(&final_url, &headers).await.ok();
            (i, path)
        }
    });
    match tokio::time::timeout(
        std::time::Duration::from_secs(90),
        iced::futures::future::join_all(futs),
    )
    .await
    {
        Err(_) => panic!("TIMEOUT tras 90s ← AQUÍ SE CUELGA EL READER"),
        Ok(results) => {
            let ok = results.iter().filter(|(_, p)| p.is_some()).count();
            eprintln!("✓ {}/{} páginas descargadas en {:?}", ok, results.len(), t0.elapsed());
        }
    }
    client.stop().await;
}
