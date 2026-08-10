use crate::core::daemon::api::MangaSourceApi;
use crate::core::daemon::rpc::{RpcException, RpcRequest, RpcResponse};
use crate::core::error::DaemonError;
use crate::core::models::{Chapter, Manga, Page, PingReply, Source};
use crate::core::xdg::Xdg;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::sync::{Mutex, oneshot};

pub struct DaemonClient {
    // Base fd (std) para poder hacer try_clone()/dup() por cada call(); el reader
    // se queda con una copia tokio del mismo socket.
    socket: Option<std::os::unix::net::UnixStream>,
    child: Option<tokio::process::Child>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, RpcException>>>>>,
    next_id: AtomicU64,
}

impl DaemonClient {
    pub fn new() -> Self {
        Self { socket: None, child: None, pending: Arc::new(Mutex::new(HashMap::new())), next_id: AtomicU64::new(1) }
    }

    pub fn default_jar_path() -> PathBuf {
        let exec_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
        let candidates: Vec<PathBuf> = vec![];
        // Espejo de defaultJarPath(): exec dir + 'daemon/build/libs/bakeneko-daemon.jar' walk-up.
        if let Some(dir) = &exec_dir {
            let mut c: Vec<PathBuf> = vec![dir.join("bakeneko-daemon.jar"), dir.join("lib/bakeneko-daemon.jar")];
            let mut cur = dir.clone();
            for _ in 0..8 {
                c.push(cur.join("daemon/build/libs/bakeneko-daemon.jar"));
                cur = cur.parent().map(|p| p.to_path_buf()).unwrap_or(cur.clone());
            }
            return c.into_iter().find(|p| p.exists()).unwrap_or(candidates.first().cloned().unwrap_or_else(|| dir.join("bakeneko-daemon.jar")));
        }
        PathBuf::from("bakeneko-daemon.jar")
    }

    async fn resolve_java() -> String {
        if let Ok(exec) = std::env::current_exe() {
            let jre = exec.parent().unwrap().join("jre/bin/java");
            if jre.exists() { return jre.to_string_lossy().into_owned(); }
        }
        if let Ok(home) = std::env::var("JAVA_HOME") {
            if !home.is_empty() {
                let f = Path::new(&home).join("bin/java");
                if f.exists() { return f.to_string_lossy().into_owned(); }
            }
        }
        "java".to_string()
    }

    pub async fn start(&mut self, jar_path: Option<&str>, java_path: Option<&str>) -> Result<(), DaemonError> {
        let jar = jar_path.map(PathBuf::from).unwrap_or_else(Self::default_jar_path);
        if !jar.exists() {
            return Err(DaemonError::Spawn(format!("No se encuentra el JAR del daemon: {}", jar.display())));
        }
        let java = match java_path {
            Some(j) => j.to_string(),
            None => Self::resolve_java().await,
        };
        let socket_path = Xdg::daemon_socket();
        // Nota: no borramos un socket previo; el daemon real (java) lo sobreescribe al
        // bindear y borrar aquí rompería un listener vivo (p. ej. el fake server del test).

        let mut child = Command::new(&java)
            .arg("-jar").arg(&jar)
            .current_dir(jar.parent().unwrap_or(Path::new(".")))
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| DaemonError::Spawn(e.to_string()))?;
        // Drena stderr del daemon en background.
        if let Some(err) = child.stderr.take() {
            tokio::spawn(async move {
                let mut r = BufReader::new(err);
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line).await {
                        Ok(0) | Err(_) => break,
                        Ok(_) => eprintln!("[daemon] {}", line.trim_end()),
                    }
                }
            });
        }
        self.child = Some(child);

        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            if let Ok(s) = tokio::net::UnixStream::connect(&socket_path).await {
                // El reader se queda con una copia tokio; `self.socket` guarda el fd
                // std para poder duplicarlo (try_clone) desde cada call().
                let std_s = s.into_std()?;
                let reader_s = std_s.try_clone()?;
                self.socket = Some(std_s);
                self.spawn_reader(tokio::net::UnixStream::from_std(reader_s)?);
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                self.stop().await;
                return Err(DaemonError::Socket("El daemon no abrió el socket tras 15s".into()));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    fn spawn_reader(&self, stream: UnixStream) {
        let pending = self.pending.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() { continue; }
                let Ok(resp) = RpcResponse::decode(&line) else { continue };
                let id = resp.id;
                let mut p = pending.lock().await;
                if let Some(id) = id {
                    if let Some(tx) = p.remove(&id) {
                        let _ = tx.send(resp.unwrap());
                    }
                }
            }
            // Socket cerrado: fallan todas las pending.
            let mut p = pending.lock().await;
            for (_, tx) in p.drain() {
                let _ = tx.send(Err(RpcException { code: -32000, message: "socket cerrado".into() }));
            }
        });
    }

    async fn call(&self, method: &str, params: Option<Value>) -> Result<Value, DaemonError> {
        let std_s = self.socket.as_ref().ok_or_else(|| DaemonError::Socket("daemon no iniciado".into()))?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let req = RpcRequest { id, method: method.to_string(), params };
        let mut sock = tokio::net::UnixStream::from_std(std_s.try_clone()?)?;
        sock.write_all(req.encode().as_bytes()).await?;
        sock.write_all(b"\n").await?;
        let res = rx.await.map_err(|_| DaemonError::Socket("socket cerrado".into()))?;
        res.map_err(DaemonError::Rpc)
    }

    pub async fn stop(&mut self) {
        self.socket = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
        }
    }
}

#[async_trait]
impl MangaSourceApi for DaemonClient {
    async fn ping(&self) -> Result<PingReply, DaemonError> {
        let v: Value = self.call("ping", None).await?;
        Ok(serde_json::from_value(v)?)
    }
    async fn list_sources(&self) -> Result<Vec<Source>, DaemonError> {
        let v = self.call("sources.list", None).await?;
        Ok(serde_json::from_value(v)?)
    }
    async fn catalog_list(&self, source: &str, offset: i32, query: Option<&str>) -> Result<Vec<Manga>, DaemonError> {
        let mut p = json!({"source": source, "offset": offset});
        if let Some(q) = query { if !q.is_empty() { p["query"] = json!(q); } }
        let v = self.call("catalog.list", Some(p)).await?;
        Ok(serde_json::from_value(v)?)
    }
    async fn manga_details(&self, source: &str, manga: &Manga) -> Result<Manga, DaemonError> {
        let blob = if manga.blob.is_empty() { serde_json::to_value(manga)? } else { Value::Object(manga.blob.clone()) };
        let v = self.call("manga.details", Some(json!({"source": source, "manga": blob}))).await?;
        Ok(serde_json::from_value(v)?)
    }
    async fn chapter_pages(&self, source: &str, chapter: &Chapter) -> Result<Vec<Page>, DaemonError> {
        let blob = if chapter.blob.is_empty() { serde_json::to_value(chapter)? } else { Value::Object(chapter.blob.clone()) };
        let v = self.call("chapter.pages", Some(json!({"source": source, "chapter": blob}))).await?;
        Ok(serde_json::from_value(v)?)
    }
    async fn page_url(&self, source: &str, page: &Page) -> Result<String, DaemonError> {
        let v = self.call("page.url", Some(json!({"source": source, "page": page}))).await?;
        Ok(v.as_str().unwrap_or_default().to_string())
    }
    async fn source_headers(&self, source: &str) -> Result<HashMap<String, String>, DaemonError> {
        let v = self.call("source.headers", Some(json!({"source": source}))).await?;
        Ok(serde_json::from_value(v)?)
    }
}
