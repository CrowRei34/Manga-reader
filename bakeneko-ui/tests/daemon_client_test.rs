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

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

async fn fake_server(path: &std::path::Path) {
    let listener = UnixListener::bind(path).unwrap();
    let (sock, _) = listener.accept().await.unwrap();
    let (rd, mut wr) = sock.into_split();
    let mut lines = BufReader::new(rd).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.contains("\"method\":\"ping\"") {
            wr.write_all(b"{\"id\":1,\"result\":{\"version\":\"1.0.0\",\"java\":\"21\"},\"jsonrpc\":\"2.0\"}\n").await.unwrap();
        }
    }
}

#[tokio::test]
async fn real_client_pings_fake_server() {
    // java shim: a script that does nothing; the socket server is the fake daemon.
    // El socket debe vivir en $XDG_RUNTIME_DIR/bakeneko/daemon.sock, que es lo que
    // Xdg::daemon_socket() devuelve con XDG_RUNTIME_DIR override.
    let sock_dir = std::env::temp_dir().join("bakeneko");
    let sock_path = sock_dir.join("daemon.sock");
    let _ = std::fs::remove_file(&sock_path);
    std::fs::create_dir_all(&sock_dir).unwrap();
    let path2 = sock_path.clone();
    tokio::spawn(async move {
        fake_server(&path2).await;
    });

    // Override XDG_RUNTIME_DIR so the client looks at our fake socket.
    std::env::set_var("XDG_RUNTIME_DIR", std::env::temp_dir());

    let client = bakeneko::core::daemon::client::DaemonClient::new();
    // jar_path fake: not used because we point start at an existing dummy file and
    // java_path to /bin/true (the daemon socket is ours, not spawned by java).
    let dummy_jar = std::env::temp_dir().join("bakeneko-dummy.jar");
    std::fs::write(&dummy_jar, b"fake").unwrap();
    client.start(Some(dummy_jar.to_str().unwrap()), Some("/bin/true")).await.unwrap();

    let p = bakeneko::core::daemon::api::MangaSourceApi::ping(&client).await.unwrap();
    assert_eq!(p.version, "1.0.0");

    client.stop().await;
    let _ = std::fs::remove_file(&sock_path);
}
