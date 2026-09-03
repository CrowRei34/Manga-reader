use crate::daemon::api::MangaSourceApi;
use crate::daemon::rpc::{RpcException, RpcRequest, RpcResponse};
use crate::error::DaemonError;
use crate::models::{Chapter, Manga, Page, PingReply, Source};
use crate::xdg::Xdg;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex as AsyncMutex};

pub struct DaemonClient {
    socket: Mutex<Option<std::os::unix::net::UnixStream>>,
    child: Mutex<Option<tokio::process::Child>>,
    solver_child: Mutex<Option<tokio::process::Child>>,
    pending: Arc<AsyncMutex<HashMap<u64, oneshot::Sender<Result<Value, RpcException>>>>>,
    next_id: AtomicU64,
}

impl DaemonClient {
    pub fn new() -> Self {
        Self {
            socket: Mutex::new(None),
            child: Mutex::new(None),
            solver_child: Mutex::new(None),
            pending: Arc::new(AsyncMutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn default_jar_path() -> PathBuf {
        if let Ok(home) = std::env::var("BAKENEKO_HOME") {
            let jar = Path::new(&home).join("bakeneko-daemon.jar");
            if jar.exists() { return jar; }
        }
        let exec_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()));
        // Espejo de defaultJarPath(): exec dir + 'daemon/build/libs/bakeneko-daemon.jar' walk-up.
        if let Some(dir) = &exec_dir {
            let mut c: Vec<PathBuf> = vec![dir.join("bakeneko-daemon.jar"), dir.join("lib/bakeneko-daemon.jar")];
            let mut cur = dir.clone();
            for _ in 0..8 {
                c.push(cur.join("daemon/build/libs/bakeneko-daemon.jar"));
                cur = cur.parent().map(|p| p.to_path_buf()).unwrap_or(cur.clone());
            }
            return c.into_iter().find(|p| p.exists()).unwrap_or_else(|| dir.join("bakeneko-daemon.jar"));
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

    pub async fn start(&self, jar_path: Option<&str>, java_path: Option<&str>) -> Result<(), DaemonError> {
        let jar = jar_path.map(PathBuf::from).unwrap_or_else(Self::default_jar_path);
        if !jar.exists() {
            eprintln!("[daemon] JAR no encontrado: {} (BAKENEKO_HOME={:?})", jar.display(), std::env::var("BAKENEKO_HOME").ok());
            return Err(DaemonError::Spawn(format!("No se encuentra el JAR del daemon: {}", jar.display())));
        }
        let java = match java_path {
            Some(j) => j.to_string(),
            None => Self::resolve_java().await,
        };
        eprintln!("[daemon] usando java={} jar={}", java, jar.display());
        let socket_path = Xdg::daemon_socket();
        // Nota: no borramos un socket previo; el daemon real (java) lo sobreescribe al
        // bindear y borrar aquí rompería un listener vivo (p. ej. el fake server del test).

        let mut cmd = Command::new(&java);
        cmd.arg("-Xmx128m")
            .arg("-XX:+UseSerialGC")
            .arg("-jar")
            .arg(&jar)
            .current_dir(jar.parent().unwrap_or(Path::new(".")));

        if let Ok(exec) = std::env::current_exe() {
            let jre = exec.parent().unwrap().join("jre/bin/java");
            if java == jre.to_string_lossy() {
                let jre_lib = exec.parent().unwrap().join("jre/lib");
                let ld_path = format!(
                    "{}/server:{}/{}",
                    jre_lib.display(),
                    jre_lib.display(),
                    std::env::var("LD_LIBRARY_PATH").unwrap_or_default()
                );
                cmd.env("LD_LIBRARY_PATH", ld_path);
            }
        }

        let mut child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            // El daemon no debe sobrevivir si la UI termina abruptamente.
            .kill_on_drop(true)
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
        {
            let mut g = self.child.lock().unwrap();
            *g = Some(child);
        }

        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            if let Ok(s) = tokio::net::UnixStream::connect(&socket_path).await {
                // El reader se queda con una copia tokio; `self.socket` guarda el fd
                // std para poder duplicarlo (try_clone) desde cada call().
                let std_s = s.into_std()?;
                let reader_s = std_s.try_clone()?;
                {
                    let mut g = self.socket.lock().unwrap();
                    *g = Some(std_s);
                }
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
        // Lock breve: sólo para duplicar el fd std desde el socket guardado.
        let std_s = {
            let g = self.socket.lock().unwrap();
            let s = g.as_ref().ok_or_else(|| DaemonError::Socket("daemon no iniciado".into()))?;
            s.try_clone()?
        };
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let req = RpcRequest { id, method: method.to_string(), params };
        // Un solo write con el '\n' incluido: con llamadas concurrentes sobre
        // fds duplicados, dos writes separados pueden entrelazar bytes de
        // requests distintos y corromper el framing NDJSON del daemon.
        let mut line = req.encode();
        line.push('\n');
        let mut sock = tokio::net::UnixStream::from_std(std_s)?;
        sock.write_all(line.as_bytes()).await?;
        // Timeout defensivo: si el daemon nunca responde, el caller recibe un
        // error visible (Reintentar en la UI) en vez de colgarse para siempre.
        let res = match tokio::time::timeout(Duration::from_secs(60), rx).await {
            Err(_) => {
                self.pending.lock().await.remove(&id);
                return Err(DaemonError::Socket(format!("timeout de 60s esperando '{method}'")));
            }
            Ok(r) => r.map_err(|_| DaemonError::Socket("socket cerrado".into()))?,
        };
        res.map_err(DaemonError::Rpc)
    }

    pub async fn stop(&self) {
        {
            let mut g = self.socket.lock().unwrap();
            *g = None;
        }
        // Take the child out of the mutex BEFORE awaiting `child.wait`, so the
        // std lock isn't held across `.await` (would block the runtime if a
        // reader locked it concurrently).
        let child_opt = { self.child.lock().unwrap().take() };
        if let Some(mut child) = child_opt {
            let _ = child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(2), child.wait()).await;
        }
        let solver_opt = { self.solver_child.lock().unwrap().take() };
        if let Some(mut sc) = solver_opt {
            let _ = sc.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(2), sc.wait()).await;
        }
    }

    /// Construye un `Arc<DaemonClient>` y arranca el daemon en un tokio task
    /// desacoplado. Devuelve el `Arc` inmediatamente para que el caller (el
    /// UI) pueda guardarlo en `AppState.daemon` y empezar a lanzar RPC tan
    /// pronto como el fondo conecte el socket. La inicialización del UI
    /// debería recibir un `Task::perform` que polla `ping`/`is_alive` y emita
    /// `Message::DaemonStarted(Ok(_))` cuando el daemon responde.
    pub fn spawn_arc(jar_path: &str) -> Arc<Self> {
        let c = Arc::new(Self::new());
        let runner = Arc::clone(&c);
        let jar = jar_path.to_string();
        tokio::spawn(async move {
            if let Err(e) = runner.start(Some(&jar), None).await {
                eprintln!("[daemon] start falló: {e}");
            }
        });
        c
    }

    /// Heartbeat del UI: indica si el socket del daemon está activo. El
    /// subscription `daemon-socket` de `app.rs` puesta a correr cada 2s emite
    /// `Message::DaemonDied` cuando éste devuelve `false`.
    pub fn is_alive(&self) -> bool {
        self.socket.lock().unwrap().is_some()
    }
}

impl Drop for DaemonClient {
    fn drop(&mut self) {
        // `Drop` es síncrono, pero `start_kill` solo envía la señal y no espera;
        // `kill_on_drop` cubre además una salida por pánico o cierre abrupto.
        if let Some(mut child) = self.child.lock().ok().and_then(|mut guard| guard.take()) {
            let _ = child.start_kill();
            eprintln!("[daemon] proceso detenido al cerrar Bakeneko");
        }
        if let Some(mut sc) = self.solver_child.lock().ok().and_then(|mut guard| guard.take()) {
            let _ = sc.start_kill();
        }
        if let Ok(mut socket) = self.socket.lock() {
            *socket = None;
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
    async fn catalog_list_filtered(&self, source: &str, offset: i32, query: Option<&str>, categories: &[String]) -> Result<Vec<Manga>, DaemonError> {
        let mut p = json!({"source": source, "offset": offset, "categories": categories});
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
