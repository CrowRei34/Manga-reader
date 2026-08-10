# Bakeneko → Rust Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the Bakeneko manga reader's Dart/Flutter app to a single Rust binary (Iced UI + tokio + rusqlite) that talks to the existing Kotlin JVM daemon via the unchanged byte-compatible JSON-RPC 2.0 protocol over a Unix Domain Socket.

**Architecture:** A single Iced application (Elm-like: `AppState` + `Message` + `update` + `subscription` + `view`). The `DaemonClient` spawns `java -jar bakeneko-daemon.jar`, connects to the UDS, and dispatches RPC responses to pending `oneshot` senders. The socket reader and download-progress emitter are Iced `Subscription`s. SQLite (rusqlite) runs behind `Arc<Mutex<Connection>>`, wrapped in `spawn_blocking`. Every feature screen is a module under `features/` with its own `Message` sub-enum.

**Tech Stack:** Rust 2021+ (edition 2021), iced 0.13 (features: `tokio`, `image`, `advanced`), tokio 1.x (features: `rt-multi-thread`, `process`, `io-util`, `net`, `sync`, `time`, `macros`, `signal`), rusqlite 0.31+ (feature: `bundled`), serde/serde_json, reqwest 0.12 (features: `json`, `gzip`, `rustls-tls`), sha2, thiserror, anyhow, async-trait.

## Global Constraints

- Daemon Kotlin code is **never modified**. RPC method names, param names, DTO JSON field names are byte-identical to the Dart client (see `daemon/src/main/kotlin/io/github/bakeneko/daemon/Methods.kt`).
- DB schema is **byte-identical** to `app/lib/core/db/schema.dart` (same table/column names, `PRAGMA user_version` migrations).
- XDG paths follow the Dart `xdg.dart`: `dataRoot=$XDG_DATA_HOME|~/.local/share + /bakeneko`, `configRoot=…/bakeneko`, `cacheRoot=…/bakeneko`, `daemonSocket=$XDG_RUNTIME_DIR|/tmp/bakeneko-<pid>/bakeneko/daemon.sock`.
- Rust field names use `camelCase` only when they must match JSON wire names (DTO serialization); internal Rust API uses `snake_case` with `#[serde(rename_all = "camelCase")]` where needed.
- The `MangaSourceApi` trait abstracts the daemon behind 7 methods so tests run without a JVM.
- AppImage must bundle: Rust binary + `bakeneko-daemon.jar` + bundled JRE. No Flutter, no libepoxy.
- Code-only corpus rule: after every commit, run `graphify update .` to keep the knowledge graph current.

## File Structure

```
bakeneko-rs/
├── Cargo.toml
├── src/
│   ├── main.rs                    # Entry: XDG ensure_dirs, Iced run()
│   ├── app.rs                     # AppState, Message (global), update, subscription, view
│   ├── theme.rs                   # Iced theme (dark/light), accent color
│   ├── core/
│   │   ├── mod.rs
│   │   ├── error.rs               # DaemonError, DbError, DownloadError (thiserror)
│   │   ├── xdg.rs                 # XDG paths + /proc uid
│   │   ├── settings.rs            # Settings load/save (settings.json)
│   │   ├── models.rs              # Manga, Chapter, Page, Source, PingReply, DownloadEntry, Category, HistoryEntry, MangaRef
│   │   ├── daemon/
│   │   │   ├── mod.rs
│   │   │   ├── api.rs             # trait MangaSourceApi
│   │   │   ├── rpc.rs             # RpcRequest/RpcResponse/RpcErr/RpcCodes
│   │   │   └── client.rs          # DaemonClient: spawn java, UDS, pending map
│   │   ├── db/
│   │   │   ├── mod.rs             # Database: open, migrate, Arc<Mutex<Connection>>, db_blocking()
│   │   │   ├── schema.rs          # const SCHEMA_SQL (identical to schema.dart)
│   │   │   └── dao/
│   │   │       ├── mod.rs
│   │   │       ├── manga_dao.rs
│   │   │       ├── chapter_dao.rs
│   │   │       ├── category_dao.rs
│   │   │       ├── history_dao.rs
│   │   │       └── download_dao.rs
│   │   ├── downloads/
│   │   │   └── mod.rs             # DownloadManager + DownloadEvent
│   │   └── net/
│   │       └── mod.rs             # image fetch + disk cache (sha256 paths)
│   └── features/
│       ├── mod.rs                 # Screen enum, view router
│       ├── shell/mod.rs           # nav rail + content wrapper
│       ├── home/mod.rs
│       ├── browse/mod.rs
│       ├── details/mod.rs
│       ├── library/mod.rs
│       ├── reader/mod.rs
│       ├── downloads/mod.rs
│       ├── settings/mod.rs
│       └── extensions/mod.rs
└── tests/                         # integration tests mirroring app/test/
    ├── xdg_test.rs
    ├── settings_test.rs
    ├── rpc_test.rs
    ├── models_test.rs
    ├── database_test.rs
    ├── dao_test.rs
    ├── download_manager_test.rs
    └── daemon_client_test.rs
```

---

### Task 1: Cargo scaffold + entry point

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/core/mod.rs`
- Create: `src/app.rs` (stub)

**Interfaces:**
- Consumes: nothing.
- Produces: `fn main()` that runs an Iced app shell; `mod core;` visible from main.

- [ ] **Step 1: Write Cargo.toml**

```toml
[package]
name = "bakeneko"
version = "0.1.0"
edition = "2021"

[dependencies]
iced = { version = "0.13", features = ["tokio", "image", "advanced"] }
tokio = { version = "1", features = ["rt-multi-thread", "process", "io-util", "net", "sync", "time", "macros", "signal"] }
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
reqwest = { version = "0.12", default-features = false, features = ["json", "gzip", "rustls-tls"] }
sha2 = "0.10"
thiserror = "1"
anyhow = "1"
async-trait = "0.1"

[profile.release]
lto = true
strip = true
```

- [ ] **Step 2: Write src/main.rs stub**

```rust
mod app;
mod core;

fn main() -> iced::Result {
    // Fase 0: todavía no lanzamos la UI real; placeholder que compila.
    println!("bakeneko-rs: placeholder");
    Ok(())
}
```

- [ ] **Step 3: Write src/core/mod.rs**

```rust
pub mod daemon;
pub mod db;
pub mod downloads;
pub mod error;
pub mod models;
pub mod net;
pub mod settings;
pub mod xdg;
```

- [ ] **Step 4: Write src/app.rs stub**

```rust
pub struct App;
```

- [ ] **Step 5: Write empty module stubs so the tree compiles**

Create these files each containing `// stub`:
- `src/core/error.rs`, `src/core/xdg.rs`, `src/core/settings.rs`, `src/core/models.rs`,
- `src/core/daemon/mod.rs`, `src/core/daemon/api.rs`, `src/core/daemon/rpc.rs`, `src/core/daemon/client.rs`,
- `src/core/db/mod.rs`, `src/core/db/schema.rs`, `src/core/db/dao/mod.rs`,
- `src/core/downloads/mod.rs`, `src/core/net/mod.rs`,
- `src/theme.rs`, `src/features/mod.rs`.

Note: `src/core/mod.rs` refers to submodules; the stubs exist so `cargo check` passes. `db/dao/mod.rs` and the `features/mod.rs` may need `pub mod` declarations as they gain files — keep declarations minimal per task.

- [ ] **Step 6: Verify build**

Run: `cargo check`
Expected: builds with 0 errors (warnings ok).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: cargo scaffold + module stubs"
```

---

### Task 2: core/xdg.rs — XDG paths

**Files:**
- Create: `src/core/xdg.rs`
- Test: `tests/xdg_test.rs`

**Interfaces:**
- Produces (public API, used by settings/db/downloads/client):

```rust
pub struct Xdg;
impl Xdg {
    pub fn home() -> String;
    pub fn data_home() -> PathBuf;    // $XDG_DATA_HOME | ~/.local/share
    pub fn config_home() -> PathBuf;
    pub fn cache_home() -> PathBuf;
    pub fn runtime_dir() -> PathBuf;  // $XDG_RUNTIME_DIR | /tmp/bakeneko-<pid>
    pub fn data_root() -> PathBuf;    // data_home/bakeneko
    pub fn config_root() -> PathBuf;
    pub fn cache_root() -> PathBuf;
    pub fn daemon_socket() -> PathBuf;
    pub fn downloads_root() -> PathBuf;
    pub fn uid() -> i64;              // /proc/self/status Uid:, fallback 1000
    pub fn ensure_dirs() -> std::io::Result<()>;
}
```

- [ ] **Step 1: Write the failing test**

```rust
// tests/xdg_test.rs
use std::env;
use std::path::PathBuf;
use bakeneko::core::xdg::Xdg;

#[test]
fn data_home_uses_env() {
    temp_env::set_var("XDG_DATA_HOME", "/tmp/xdgtest/data");
    assert_eq!(Xdg::data_home(), PathBuf::from("/tmp/xdgtest/data"));
}

#[test]
fn data_home_defaults_to_local_share() {
    temp_env::set_var("XDG_DATA_HOME", "");
    temp_env::set_var("HOME", "/tmp/homedir");
    assert_eq!(Xdg::data_home(), PathBuf::from("/tmp/homedir/.local/share"));
}

#[test]
fn daemon_socket_under_runtime_bakeneko() {
    temp_env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
    assert_eq!(
        Xdg::daemon_socket(),
        PathBuf::from("/run/user/1000/bakeneko/daemon.sock")
    );
}

#[test]
fn data_root_appends_bakeneko() {
    temp_env::set_var("XDG_DATA_HOME", "/tmp/xdgtest/data");
    assert_eq!(Xdg::data_root(), PathBuf::from("/tmp/xdgtest/data/bakeneko"));
}
```

Note: add `temp-env` to `[dev-dependencies]`:

```toml
[dev-dependencies]
temp-env = "0.3"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test xdg_test`
Expected: FAIL (module has no `data_home` etc.)

- [ ] **Step 3: Implement xdg.rs**

```rust
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Xdg;

impl Xdg {
    fn home() -> String {
        env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
    }

    fn dir_or_default(var: &str, default: PathBuf) -> PathBuf {
        match env::var(var) {
            Ok(v) if !v.is_empty() => PathBuf::from(v),
            _ => default,
        }
    }

    pub fn data_home() -> PathBuf {
        Self::dir_or_default("XDG_DATA_HOME", PathBuf::from(format!("{}/.local/share", Self::home())))
    }
    pub fn config_home() -> PathBuf {
        Self::dir_or_default("XDG_CONFIG_HOME", PathBuf::from(format!("{}/.config", Self::home())))
    }
    pub fn cache_home() -> PathBuf {
        Self::dir_or_default("XDG_CACHE_HOME", PathBuf::from(format!("{}/.cache", Self::home())))
    }
    pub fn runtime_dir() -> PathBuf {
        Self::dir_or_default("XDG_RUNTIME_DIR", PathBuf::from(format!("/tmp/bakeneko-{}", Self::uid())))
    }

    pub fn data_root() -> PathBuf { Self::data_home().join("bakeneko") }
    pub fn config_root() -> PathBuf { Self::config_home().join("bakeneko") }
    pub fn cache_root() -> PathBuf { Self::cache_home().join("bakeneko") }
    pub fn daemon_socket() -> PathBuf { Self::runtime_dir().join("bakeneko").join("daemon.sock") }
    pub fn downloads_root() -> PathBuf { Self::data_root().join("downloads") }

    pub fn uid() -> i64 {
        // Espejo del Dart: lee /proc/self/status Uid:, fallback 1000.
        let Ok(contents) = fs::read_to_string("/proc/self/status") else {
            return 1000;
        };
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(first) = rest.split_whitespace().next() {
                    return first.parse().unwrap_or(1000);
                }
            }
        }
        1000
    }

    pub fn ensure_dirs() -> std::io::Result<()> {
        for d in [Self::data_root(), Self::config_root(), Self::cache_root(), Self::downloads_root()] {
            fs::create_dir_all(d)?;
        }
        fs::create_dir_all(Self::runtime_dir().join("bakeneko"))?;
        Ok(())
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test xdg_test`
Expected: 4 PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/core/xdg.rs tests/xdg_test.rs
git commit -m "feat(core): XDG paths + uid via /proc (espejo de xdg.dart)"
```

---

### Task 3: core/settings.rs — settings.json load/save

**Files:**
- Create: `src/core/settings.rs`
- Test: `tests/settings_test.rs`

**Interfaces:**
- Produces:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,             // "dark" | "light" | "system"
    pub accent: String,            // hex
    pub default_source: Option<String>,
    pub download_concurrency: u32,
    pub library_view: String,      // "grid" | "list"
}
impl Default for Settings { ... } // dark, "#7c5cbf", None, 2, "grid"

pub fn load() -> Settings;                       // from Xdg::config_root()/settings.json
pub fn save(s: &Settings) -> std::io::Result<()>; // atomic: tmp + rename
```

- [ ] **Step 1: Write the failing test**

```rust
// tests/settings_test.rs
use bakeneko::core::settings::{load, save, Settings};
use std::env;
use std::path::PathBuf;

#[test]
fn roundtrip_preserves_values() {
    temp_env::set_var("XDG_CONFIG_HOME", "/tmp/settest/config");
    let s = Settings { theme: "light".into(), accent: "#ff0000".into(), default_source: Some("MANGADEX".into()), download_concurrency: 4, library_view: "list".into() };
    save(&s).unwrap();
    let loaded = load();
    assert_eq!(loaded.theme, "light");
    assert_eq!(loaded.accent, "#ff0000");
    assert_eq!(loaded.default_source, Some("MANGADEX".to_string()));
    assert_eq!(loaded.download_concurrency, 4);
    assert_eq!(loaded.library_view, "list");
}

#[test]
fn missing_file_yields_default() {
    temp_env::set_var("XDG_CONFIG_HOME", "/tmp/settest/empty");
    let s = load();
    assert_eq!(s.theme, "dark");
    assert_eq!(s.download_concurrency, 2);
}

#[test]
fn corrupt_file_yields_default() {
    temp_env::set_var("XDG_CONFIG_HOME", "/tmp/settest/corrupt");
    std::fs::create_dir_all(PathBuf::from("/tmp/settest/corrupt/bakeneko")).unwrap();
    std::fs::write(PathBuf::from("/tmp/settest/corrupt/bakeneko/settings.json"), "{not json").unwrap();
    let s = load();
    assert_eq!(s.theme, "dark");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test settings_test`
Expected: FAIL.

- [ ] **Step 3: Implement settings.rs**

```rust
use crate::core::xdg::Xdg;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: String,
    pub accent: String,
    pub default_source: Option<String>,
    pub download_concurrency: u32,
    pub library_view: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: "dark".into(), accent: "#7c5cbf".into(), default_source: None, download_concurrency: 2, library_view: "grid".into() }
    }
}

fn settings_path() -> PathBuf { Xdg::config_root().join("settings.json") }

pub fn load() -> Settings {
    let path = settings_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) -> std::io::Result<()> {
    let path = settings_path();
    fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension("json.tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(serde_json::to_string_pretty(s).unwrap().as_bytes())?;
    f.sync_all()?;
    fs::rename(&tmp, &path)?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test settings_test`
Expected: 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/settings.rs tests/settings_test.rs
git commit -m "feat(core): settings.json load/save con serde (espejo de settings.dart)"
```

---

### Task 4: core/daemon/rpc.rs — JSON-RPC framing

**Files:**
- Create: `src/core/daemon/rpc.rs`
- Test: `tests/rpc_test.rs`

**Interfaces:**
- Produces:

```rust
pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcRequest { pub id: u64, pub method: String, pub params: Option<serde_json::Value> }
impl RpcRequest {
    pub fn encode(&self) -> String;  // {"id":..,"method":..,"params":..,"jsonrpc":"2.0"} \n-free
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcErr { pub code: i32, pub message: String }

#[derive(Debug, Clone)]
pub struct RpcResponse { pub id: Option<u64>, pub result: Option<serde_json::Value>, pub error: Option<RpcErr> }
impl RpcResponse {
    pub fn decode(line: &str) -> Result<Self, serde_json::Error>;
    pub fn is_ok(&self) -> bool;
    pub fn unwrap(self) -> Result<serde_json::Value, RpcException>;
}

#[derive(Debug, thiserror::Error)]
#[error("RpcException({code}): {message}")]
pub struct RpcException { pub code: i32, pub message: String }
```

- [ ] **Step 1: Write the failing test**

```rust
// tests/rpc_test.rs
use bakeneko::core::daemon::rpc::{RpcErr, RpcRequest, RpcResponse, RpcException, codes};

#[test]
fn request_encodes_jsonrpc_field() {
    let req = RpcRequest { id: 7, method: "ping".into(), params: None };
    let s = req.encode();
    assert!(s.contains("\"id\":7"));
    assert!(s.contains("\"method\":\"ping\""));
    assert!(s.contains("\"jsonrpc\":\"2.0\""));
    assert!(!s.contains('\n'));
}

#[test]
fn response_decode_ok() {
    let line = r#"{"id":7,"result":{"version":"1.0.0","java":"21"},"jsonrpc":"2.0"}"#;
    let r = RpcResponse::decode(line).unwrap();
    assert!(r.is_ok());
    assert_eq!(r.id, Some(7));
    assert_eq!(r.result.unwrap()["version"], "1.0.0");
}

#[test]
fn response_decode_error_and_unwrap() {
    let line = r#"{"id":7,"error":{"code":-32602,"message":"falta source"},"jsonrpc":"2.0"}"#;
    let r = RpcResponse::decode(line).unwrap();
    assert!(!r.is_ok());
    let err = r.unwrap().unwrap_err();
    assert_eq!(err.code, codes::INVALID_PARAMS);
    assert_eq!(err.message, "falta source");
}

#[test]
fn error_omits_id_when_null() {
    let line = r#"{"error":{"code":-32700,"message":"JSON inválido"},"jsonrpc":"2.0"}"#;
    let r = RpcResponse::decode(line).unwrap();
    assert_eq!(r.id, None);
}

#[test]
fn rpc_exception_to_string() {
    let e = RpcException { code: -32601, message: "método desconocido".into() };
    assert!(e.to_string().contains("método desconocido"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test rpc_test`
Expected: FAIL.

- [ ] **Step 3: Implement rpc.rs**

```rust
use serde::{Deserialize, Serialize};

pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

#[derive(Debug, Clone, Serialize)]
pub struct RpcRequest {
    pub id: u64,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl RpcRequest {
    pub fn encode(&self) -> String {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), serde_json::json!(self.id));
        obj.insert("method".into(), serde_json::json!(self.method));
        if let Some(p) = &self.params {
            obj.insert("params".into(), p.clone());
        }
        obj.insert("jsonrpc".into(), serde_json::json!("2.0"));
        serde_json::Value::Object(obj).to_string()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcErr { pub code: i32, pub message: String }

#[derive(Debug, Clone)]
pub struct RpcResponse {
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<RpcErr>,
}

impl RpcResponse {
    pub fn decode(line: &str) -> Result<Self, serde_json::Error> {
        #[derive(Deserialize)]
        struct Raw {
            id: Option<serde_json::Value>,
            result: Option<serde_json::Value>,
            error: Option<RpcErr>,
        }
        let raw: Raw = serde_json::from_str(line)?;
        let id = match raw.id {
            Some(serde_json::Value::Number(n)) => n.as_u64(),
            _ => None,
        };
        Ok(RpcResponse { id, result: raw.result, error: raw.error })
    }

    pub fn is_ok(&self) -> bool { self.error.is_none() }

    pub fn unwrap(self) -> Result<serde_json::Value, RpcException> {
        match self.error {
            Some(e) => Err(RpcException { code: e.code, message: e.message }),
            None => Ok(self.result.unwrap_or(serde_json::Value::Null)),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("RpcException({code}): {message}")]
pub struct RpcException { pub code: i32, pub message: String }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test rpc_test`
Expected: 5 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/daemon/rpc.rs tests/rpc_test.rs
git commit -m "feat(daemon): JSON-RPC framing byte-compatible (espejo de rpc.dart)"
```

---

### Task 5: core/models.rs — DTOs byte-compatibles

**Files:**
- Create: `src/core/models.rs`
- Test: `tests/models_test.rs`

**Interfaces:**
- Produces (public types; `blob` is the opaque JSON for daemon round-trip):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manga {
    pub source: String,
    pub url: String,
    pub title: String,
    #[serde(rename = "publicUrl")] pub public_url: Option<String>,
    pub rating: f32,
    #[serde(rename = "isNsfw")] pub is_nsfw: bool,
    #[serde(rename = "coverUrl")] pub cover_url: Option<String>,
    #[serde(rename = "largeCoverUrl")] pub large_cover_url: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub state: Option<String>,
    pub chapters: Vec<Chapter>,
    #[serde(skip)] pub blob: serde_json::Map<String, serde_json::Value>,
}
impl Manga { pub fn key(&self) -> String; pub fn blob_json(&self) -> String; }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter { pub source: String, pub url: String, pub title: String, pub number: f32,
    pub volume: i32, pub scanlator: Option<String>, #[serde(rename="uploadDate")] pub upload_date: i64,
    pub branch: Option<String>, #[serde(skip)] pub blob: serde_json::Map<String, serde_json::Value>,
    #[serde(skip)] pub read: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page { pub source: String, pub url: String, pub preview: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source { pub id: String, pub name: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingReply { pub version: String, pub java: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState { Idle, Queued, Downloading, Done, Error }

#[derive(Debug, Clone)]
pub struct DownloadEntry { pub manga_id: i64, pub chapter_url: String, pub state: DownloadState, pub total_pages: i32, pub done_pages: i32 }

#[derive(Debug, Clone)]
pub struct Category { pub id: Option<i64>, pub name: String, pub color: String, pub auto_download: bool, pub created_at: i64 }

#[derive(Debug, Clone)]
pub struct HistoryEntry { pub manga: Manga, pub chapter_index: i32, pub page_index: i32, pub updated_at: i64 }

#[derive(Debug, Clone)]
pub struct MangaRef { pub source: String, pub url: String, pub title: String }
```

- [ ] **Step 1: Write the failing test**

```rust
// tests/models_test.rs
use bakeneko::core::models::*;
use serde_json::json;

#[test]
fn manga_deserializes_with_camelcase() {
    let j = json!({
        "source": "MANGADEX", "url": "/title/abc", "title": "One Piece",
        "publicUrl": "https://mangadex.org/title/abc",
        "rating": 4.5, "isNsfw": false, "coverUrl": "http://c/1.jpg",
        "largeCoverUrl": "http://c/1l.jpg", "description": "d", "authors": ["Oda"],
        "state": "ONGOING", "chapters": []
    });
    let m: Manga = serde_json::from_value(j).unwrap();
    assert_eq!(m.source, "MANGADEX");
    assert_eq!(m.public_url.as_deref(), Some("https://mangadex.org/title/abc"));
    assert_eq!(m.rating, 4.5);
    assert_eq!(m.authors, vec!["Oda".to_string()]);
    assert_eq!(m.key(), "MANGADEX|/title/abc");
}

#[test]
fn manga_defaults_for_missing_optional() {
    let j = json!({"source": "MANGASEE", "url": "/u", "title": "T"});
    let m: Manga = serde_json::from_value(j).unwrap();
    assert_eq!(m.rating, 0.0);
    assert!(!m.is_nsfw);
    assert!(m.authors.is_empty());
    assert!(m.chapters.is_empty());
}

#[test]
fn chapter_upload_date_roundtrip() {
    let j = json!({"source":"MANGADEX","url":"/c1","title":"Cap 1","number":1.5,"volume":2,"uploadDate":1700000000000});
    let c: Chapter = serde_json::from_value(j).unwrap();
    assert_eq!(c.number, 1.5);
    assert_eq!(c.upload_date, 1700000000000);
}

#[test]
fn page_roundtrip() {
    let p = Page { source: "S".into(), url: "http://img/1.jpg".into(), preview: None };
    let v = serde_json::to_value(&p).unwrap();
    let back: Page = serde_json::from_value(v).unwrap();
    assert_eq!(back.url, "http://img/1.jpg");
}

#[test]
fn download_state_serde_lowercase() {
    let v = serde_json::json!("downloading");
    let s: DownloadState = serde_json::from_value(v).unwrap();
    assert_eq!(s, DownloadState::Downloading);
    assert_eq!(serde_json::to_value(&s).unwrap(), serde_json::json!("downloading"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test models_test`
Expected: FAIL (types don't exist yet).

- [ ] **Step 3: Implement models.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manga {
    pub source: String,
    pub url: String,
    pub title: String,
    #[serde(rename = "publicUrl")] pub public_url: Option<String>,
    #[serde(default)] pub rating: f32,
    #[serde(rename = "isNsfw", default)] pub is_nsfw: bool,
    #[serde(rename = "coverUrl")] pub cover_url: Option<String>,
    #[serde(rename = "largeCoverUrl")] pub large_cover_url: Option<String>,
    pub description: Option<String>,
    #[serde(default)] pub authors: Vec<String>,
    pub state: Option<String>,
    #[serde(default)] pub chapters: Vec<Chapter>,
    #[serde(skip)] pub blob: serde_json::Map<String, serde_json::Value>,
}

impl Manga {
    pub fn key(&self) -> String { format!("{}|{}", self.source, self.url) }
    pub fn blob_json(&self) -> String {
        if self.blob.is_empty() { serde_json::json!({}).to_string() } else { serde_json::Value::Object(self.blob.clone()).to_string() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub source: String,
    pub url: String,
    pub title: String,
    #[serde(default)] pub number: f32,
    #[serde(default)] pub volume: i32,
    pub scanlator: Option<String>,
    #[serde(rename = "uploadDate", default)] pub upload_date: i64,
    pub branch: Option<String>,
    #[serde(skip)] pub blob: serde_json::Map<String, serde_json::Value>,
    #[serde(skip)] pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub source: String,
    pub url: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source { pub id: String, pub name: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingReply { pub version: String, pub java: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadState { Idle, Queued, Downloading, Done, Error }

#[derive(Debug, Clone)]
pub struct DownloadEntry {
    pub manga_id: i64,
    pub chapter_url: String,
    pub state: DownloadState,
    pub total_pages: i32,
    pub done_pages: i32,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub color: String,
    pub auto_download: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub manga: Manga,
    pub chapter_index: i32,
    pub page_index: i32,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct MangaRef { pub source: String, pub url: String, pub title: String }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test models_test`
Expected: 5 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/models.rs tests/models_test.rs
git commit -m "feat(core): DTOs byte-compatibles con el daemon (espejo de models.dart)"
```

---

### Task 6: core/daemon/api.rs — trait MangaSourceApi

**Files:**
- Create: `src/core/daemon/api.rs`
- Test: `tests/daemon_client_test.rs` (mock impl)

**Interfaces:**
- Produces (async-trait; the production impl is `DaemonClient` in Task 7):

```rust
use crate::core::models::{Manga, Page, PingReply, Source};
use crate::core::error::DaemonError;
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
```

- [ ] **Step 1: Write the failing test (mock impl validates the trait is usable)**

```rust
// tests/daemon_client_test.rs
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test daemon_client_test`
Expected: FAIL (no trait).

- [ ] **Step 3: Implement api.rs** (trait code above). Also add a unit test in `api.rs` (or keep it only in tests/).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test daemon_client_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/daemon/api.rs tests/daemon_client_test.rs
git commit -m "feat(daemon): trait MangaSourceApi para testear sin JVM"
```

---

### Task 7: core/daemon/client.rs — DaemonClient real

**Files:**
- Create: `src/core/daemon/client.rs`
- Modify: `src/core/error.rs` (add `DaemonError`)
- Test: `tests/daemon_client_test.rs` (extend with an in-process fake socket server)

**Interfaces:**
- Consumes: `MangaSourceApi` (Task 6), `RpcRequest`/`RpcResponse`/`RpcException` (Task 4), `models` (Task 5).
- Produces:

```rust
pub struct DaemonClient { /* private */ }
impl DaemonClient {
    pub fn new() -> Self;
    pub async fn start(&mut self, jar_path: Option<&str>, java_path: Option<&str>) -> Result<(), DaemonError>;
    pub async fn stop(&mut self);
    pub fn default_jar_path() -> PathBuf;
}
impl MangaSourceApi for DaemonClient { /* the 7 methods via call() */ }
```

Internals:
- `socket: Option<tokio::net::UnixStream>`, `child: Option<tokio::process::Child>`,
  `pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<Result<serde_json::Value, RpcException>>>>>`,
  `next_id: AtomicU64`.
- `async fn call(&self, method: &str, params: Option<Value>) -> Result<Value, DaemonError>`:
  build RpcRequest, register oneshot, write `encode() + "\n"`, await receiver, map RpcException → DaemonError::Rpc.

- [ ] **Step 1: Add DaemonError to error.rs**

```rust
use crate::core::daemon::rpc::RpcException;

#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("DaemonException: {0}")]
    Spawn(String),
    #[error("DaemonException: {0}")]
    Socket(String),
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcException),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

- [ ] **Step 2: Write the failing test — fake Unix socket server**

In-process test: create a UnixListener on a temp path, spawn a task that accepts one connection and replies to a `ping` line, then exercise `DaemonClient::start` pointed at a fake `java` script.

```rust
// tests/daemon_client_test.rs (append)
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

async fn fake_server(path: &std::path::Path) {
    let listener = UnixListener::bind(path).unwrap();
    let (mut sock, _) = listener.accept().await.unwrap();
    let mut lines = BufReader::new(sock.try_clone().unwrap()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.contains("\"method\":\"ping\"") {
            sock.write_all(b"{\"id\":1,\"result\":{\"version\":\"1.0.0\",\"java\":\"21\"},\"jsonrpc\":\"2.0\"}\n").await.unwrap();
        }
    }
}

#[tokio::test]
async fn real_client_pings_fake_server() {
    // java shim: a script that does nothing; the socket server is the fake daemon.
    let sock_path = std::env::temp_dir().join(format!("bakeneko-test-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&sock_path);
    let path2 = sock_path.clone();
    tokio::spawn(fake_server(&path2));

    // Override XDG_RUNTIME_DIR so the client looks at our fake socket.
    temp_env::set_var("XDG_RUNTIME_DIR", std::env::temp_dir());
    std::fs::create_dir_all(std::env::temp_dir().join("bakeneko")).unwrap();

    let mut client = bakeneko::core::daemon::client::DaemonClient::new();
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
```

Note: this requires `DaemonClient::start` to connect to `Xdg::daemon_socket()` which the test overrides via `XDG_RUNTIME_DIR`. The java shim `/bin/true` just keeps the client happy that a process exists; the actual daemon is our fake server. The client polls `_try_connect` until the socket appears.

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --test daemon_client_test -- --nocapture`
Expected: FAIL (no DaemonClient).

- [ ] **Step 4: Implement client.rs**

```rust
use crate::core::daemon::api::MangaSourceApi;
use crate::core::daemon::rpc::{RpcException, RpcRequest, RpcResponse};
use crate::core::error::DaemonError;
use crate::core::models::{Chapter, Manga, Page, PingReply, Source};
use crate::core::xdg::Xdg;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
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
    socket: Option<UnixStream>,
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
        if socket_path.exists() { let _ = std::fs::remove_file(&socket_path); }

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
                self.socket = Some(s.clone());
                self.spawn_reader(s);
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
        let sock = self.socket.as_ref().ok_or_else(|| DaemonError::Socket("daemon no iniciado".into()))?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        let req = RpcRequest { id, method: method.to_string(), params };
        let mut sock = sock.try_clone().map_err(|e| DaemonError::Io(e))?;
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
```

Note: `DaemonError` needs a `From<serde_json::Error>` variant to use `?` on `serde_json::from_value`. Add to error.rs:

```rust
#[error("JSON error: {0}")]
Json(#[from] serde_json::Error),
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test daemon_client_test`
Expected: 2 PASS (mock + real fake-server ping).

- [ ] **Step 6: Commit**

```bash
git add src/core/error.rs src/core/daemon/client.rs tests/daemon_client_test.rs
git commit -m "feat(daemon): DaemonClient real (spawn java + UDS + pending map)"
```

---

### Task 8: core/db — schema + Database (migraciones)

**Files:**
- Create: `src/core/db/schema.rs`, `src/core/db/mod.rs`
- Test: `tests/database_test.rs`

**Interfaces:**
- Produces:

```rust
pub const SCHEMA_SQL: &str;  // idéntico a schema.dart

pub struct Database { conn: Arc<Mutex<rusqlite::Connection>> }
impl Database {
    pub fn open(path: Option<&Path>) -> Result<Self, DbError>;  // None => :memory:
    pub fn migrate(&self) -> Result<(), DbError>;
    pub fn user_version(&self) -> Result<i32, DbError>;
    pub fn connection(&self) -> Arc<Mutex<Connection>>;
}
pub async fn db_blocking<F, T>(db: Arc<Mutex<Connection>>, f: F) -> Result<T, DbError>
where F: FnOnce(&mut Connection) -> Result<T, DbError> + Send + 'static, T: Send + 'static;
```

`DbError` lives in `error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQL error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("join error: {0}")]
    Join(String),
}
```

- [ ] **Step 1: Write the failing test**

```rust
// tests/database_test.rs
use bakeneko::core::db::{Database, SCHEMA_SQL};
use rusqlite::Connection;

#[test]
fn schema_matches_dart_tables() {
    for table in ["manga", "chapter", "category", "manga_category", "history", "download"] {
        assert!(SCHEMA_SQL.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")),
            "falta tabla {table}");
    }
}

#[test]
fn open_memory_and_migrate_is_idempotent() {
    let db = Database::open(None).unwrap();
    db.migrate().unwrap();
    db.migrate().unwrap(); // idempotente
    assert_eq!(db.user_version().unwrap(), 1);
}

#[test]
fn open_file_creates_schema() {
    let dir = std::env::temp_dir().join("bakeneko-db-test");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("test.sqlite");
    let _ = std::fs::remove_file(&p);
    {
        let db = Database::open(Some(&p)).unwrap();
        db.migrate().unwrap();
    }
    let conn = Connection::open(&p).unwrap();
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table'", [], |r| r.get(0)).unwrap();
    assert!(n >= 6);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test database_test`
Expected: FAIL.

- [ ] **Step 3: Implement schema.rs** — copy the SQL verbatim from `app/lib/core/db/schema.dart` (the `schemaSql` raw string, which you read earlier in this session):

```rust
pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS manga (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    cover_url TEXT,
    description TEXT,
    blob_json TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    library INTEGER NOT NULL DEFAULT 0,
    UNIQUE(source, url)
);

CREATE TABLE IF NOT EXISTS chapter (
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    name TEXT NOT NULL,
    number REAL NOT NULL DEFAULT 0,
    blob_json TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (manga_id, url)
);

CREATE TABLE IF NOT EXISTS category (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL,
    auto_download INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS manga_category (
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    PRIMARY KEY (manga_id, category_id)
);

CREATE TABLE IF NOT EXISTS history (
    manga_id INTEGER NOT NULL PRIMARY KEY REFERENCES manga(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    page_index INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS download (
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    chapter_url TEXT NOT NULL,
    state TEXT NOT NULL,
    total_pages INTEGER NOT NULL DEFAULT 0,
    done_pages INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (manga_id, chapter_url)
);

CREATE INDEX IF NOT EXISTS idx_history_updated ON history(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_chapter_manga ON chapter(manga_id);
"#;
```

- [ ] **Step 4: Implement mod.rs**

```rust
pub mod dao;
pub mod schema;

use crate::core::error::DbError;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use schema::SCHEMA_SQL;

pub struct Database { conn: Arc<Mutex<Connection>> }

impl Database {
    pub fn open(path: Option<&Path>) -> Result<Self, DbError> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn migrate(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        let v: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if v < 1 {
            conn.execute_batch(SCHEMA_SQL)?;
            conn.pragma_update(None, "user_version", 1)?;
        }
        Ok(())
    }

    pub fn user_version(&self) -> Result<i32, DbError> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> { self.conn.clone() }
}

pub async fn db_blocking<F, T>(db: Arc<Mutex<Connection>>, f: F) -> Result<T, DbError>
where F: FnOnce(&mut Connection) -> Result<T, DbError> + Send + 'static,
      T: Send + 'static {
    tokio::task::spawn_blocking(move || {
        let mut conn = db.lock().unwrap();
        f(&mut conn)
    })
    .await
    .map_err(|e| DbError::Join(e.to_string()))?
}
```

Note: `dao/mod.rs` must declare the 5 DAO modules — create it in Task 9.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test database_test`
Expected: 3 PASS.

- [ ] **Step 6: Commit**

```bash
git add src/core/error.rs src/core/db/schema.rs src/core/db/mod.rs tests/database_test.rs
git commit -m "feat(db): schema idéntico + migraciones user_version + db_blocking"
```

---

### Task 9: core/db/dao — 5 DAOs

**Files:**
- Create: `src/core/db/dao/mod.rs`, `manga_dao.rs`, `chapter_dao.rs`, `category_dao.rs`, `history_dao.rs`, `download_dao.rs`
- Test: `tests/dao_test.rs`

**Interfaces:**
- Consumes: `Database` (Task 8), `models` (Task 5).
- Produces (all take `&Connection`):

```rust
pub mod manga_dao {
    pub fn upsert(conn: &Connection, m: &Manga, added_at: i64) -> Result<i64, DbError>; // id del manga
    pub fn get_by_key(conn: &Connection, source: &str, url: &str) -> Result<Option<Manga>, DbError>;
    pub fn get_by_id(conn: &Connection, id: i64) -> Result<Option<Manga>, DbError>;
    pub fn get_id_by_key(conn: &Connection, source: &str, url: &str) -> Result<i64, DbError>; // Err(QueryReturnedNoRows) si no existe
    pub fn list_library(conn: &Connection) -> Result<Vec<Manga>, DbError>;
    pub fn set_library_flag(conn: &Connection, id: i64, in_library: bool) -> Result<(), DbError>;
    pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError>;
}
pub mod chapter_dao {
    pub fn replace_for_manga(conn: &Connection, manga_id: i64, chapters: &[Chapter]) -> Result<(), DbError>;
    pub fn list_for_manga(conn: &Connection, manga_id: i64) -> Result<Vec<Chapter>, DbError>;
    pub fn mark_read(conn: &Connection, manga_id: i64, url: &str, read: bool) -> Result<(), DbError>;
}
pub mod category_dao {
    pub fn list(conn: &Connection) -> Result<Vec<Category>, DbError>;
    pub fn add(conn: &Connection, name: &str, color: &str) -> Result<i64, DbError>;
    pub fn rename(conn: &Connection, id: i64, name: &str) -> Result<(), DbError>;
    pub fn set_color(conn: &Connection, id: i64, color: &str) -> Result<(), DbError>;
    pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError>;
    pub fn assign(conn: &Connection, manga_id: i64, category_id: i64) -> Result<(), DbError>;
    pub fn unassign(conn: &Connection, manga_id: i64, category_id: i64) -> Result<(), DbError>;
    pub fn for_manga(conn: &Connection, manga_id: i64) -> Result<Vec<Category>, DbError>;
}
pub mod history_dao {
    pub fn upsert(conn: &Connection, manga_id: i64, chapter_index: i32, page_index: i32, updated_at: i64) -> Result<(), DbError>;
    pub fn recent(conn: &Connection, limit: i64) -> Result<Vec<(i64, i32, i32, i64)>, DbError>;
    pub fn delete(conn: &Connection, manga_id: i64) -> Result<(), DbError>;
}
pub mod download_dao {
    pub fn upsert(conn: &Connection, manga_id: i64, chapter_url: &str, state: DownloadState) -> Result<(), DbError>;
    pub fn list(conn: &Connection) -> Result<Vec<DownloadEntry>, DbError>;
    pub fn list_by_state(conn: &Connection, state: DownloadState) -> Result<Vec<DownloadEntry>, DbError>;
    pub fn update_progress(conn: &Connection, manga_id: i64, chapter_url: &str, done: i32, total: i32) -> Result<(), DbError>;
    pub fn set_state(conn: &Connection, manga_id: i64, chapter_url: &str, state: DownloadState) -> Result<(), DbError>;
}
```

Row→model mapping: `Manga`/`Chapter` restore `blob` by parsing `blob_json` via `serde_json::from_str`; `read`/library flags come from columns. `blob_json = m.blob_json()` (Task 5).

- [ ] **Step 1: Write the failing test**

```rust
// tests/dao_test.rs
use bakeneko::core::db::Database;
use bakeneko::core::db::dao::{manga_dao, chapter_dao, history_dao, category_dao, download_dao};
use bakeneko::core::models::{Chapter, DownloadState, Manga};

fn setup() -> Connection {
    let db = Database::open(None).unwrap();
    db.migrate().unwrap();
    let conn = db.connection().lock().unwrap().clone();
    conn
}

fn sample_manga(url: &str) -> Manga {
    Manga { source: "MANGADEX".into(), url: url.into(), title: "T".into(), public_url: None,
        rating: 0.0, is_nsfw: false, cover_url: None, large_cover_url: None,
        description: None, authors: vec![], state: None, chapters: vec![], blob: Default::default() }
}

#[test]
fn manga_upsert_and_get_by_key() {
    let conn = setup();
    let m = sample_manga("/u1");
    let id = manga_dao::upsert(&conn, &m, 1700000000).unwrap();
    let got = manga_dao::get_by_key(&conn, "MANGADEX", "/u1").unwrap().unwrap();
    assert_eq!(got.title, "T");
    let id2 = manga_dao::upsert(&conn, &m, 1700000000).unwrap();
    assert_eq!(id, id2); // idempotente por UNIQUE(source,url)
}

#[test]
fn library_flag_roundtrip() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/lib"), 0).unwrap();
    manga_dao::set_library_flag(&conn, id, true).unwrap();
    let lib = manga_dao::list_library(&conn).unwrap();
    assert_eq!(lib.len(), 1);
}

#[test]
fn chapters_replace_and_mark_read() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/ch"), 0).unwrap();
    let ch = Chapter { source: "MANGADEX".into(), url: "/c1".into(), title: "Cap 1".into(), number: 1.0,
        volume: 1, scanlator: None, upload_date: 0, branch: None, blob: Default::default(), read: false };
    chapter_dao::replace_for_manga(&conn, id, &[ch]).unwrap();
    let list = chapter_dao::list_for_manga(&conn, id).unwrap();
    assert_eq!(list.len(), 1);
    chapter_dao::mark_read(&conn, id, "/c1", true).unwrap();
    assert!(chapter_dao::list_for_manga(&conn, id).unwrap()[0].read);
}

#[test]
fn history_upsert_recent() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/h"), 0).unwrap();
    history_dao::upsert(&conn, id, 2, 5, 200).unwrap();
    history_dao::upsert(&conn, id, 3, 0, 300).unwrap(); // misma manga, último gana
    let rec = history_dao::recent(&conn, 10).unwrap();
    assert_eq!(rec.len(), 1);
    assert_eq!(rec[0], (id, 3, 0, 300));
}

#[test]
fn categories_and_assignment() {
    let conn = setup();
    let cid = category_dao::add(&conn, "Favoritos", "#ff0000").unwrap();
    let id = manga_dao::upsert(&conn, &sample_manga("/cat"), 0).unwrap();
    category_dao::assign(&conn, id, cid).unwrap();
    let cats = category_dao::for_manga(&conn, id).unwrap();
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0].name, "Favoritos");
}

#[test]
fn download_states_transition() {
    let conn = setup();
    let id = manga_dao::upsert(&conn, &sample_manga("/dl"), 0).unwrap();
    download_dao::upsert(&conn, id, "/c1", DownloadState::Queued).unwrap();
    download_dao::update_progress(&conn, id, "/c1", 2, 10).unwrap();
    let e = download_dao::list(&conn).unwrap();
    assert_eq!(e[0].done_pages, 2);
    assert_eq!(e[0].total_pages, 10);
    download_dao::set_state(&conn, id, "/c1", DownloadState::Done).unwrap();
    assert_eq!(download_dao::list_by_state(&conn, DownloadState::Done).unwrap().len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test dao_test`
Expected: FAIL.

- [ ] **Step 3: Implement the 5 DAOs**

`dao/mod.rs`:

```rust
pub mod category_dao;
pub mod chapter_dao;
pub mod download_dao;
pub mod history_dao;
pub mod manga_dao;
```

`manga_dao.rs` (representative; the rest follow the same `execute`/`query_row` pattern):

```rust
use crate::core::error::DbError;
use crate::core::models::Manga;
use rusqlite::{params, Connection};

fn row_to_manga(row: &rusqlite::Row) -> rusqlite::Result<Manga> {
    let blob_json: String = row.get(6)?;
    let blob: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&blob_json).unwrap_or_default();
    Ok(Manga {
        source: row.get(1)?, url: row.get(2)?, title: row.get(3)?,
        public_url: None, rating: 0.0, is_nsfw: false,
        cover_url: row.get(4)?, large_cover_url: None, description: row.get(5)?,
        authors: vec![], state: None, chapters: vec![], blob,
    })
}

pub fn upsert(conn: &Connection, m: &Manga, added_at: i64) -> Result<i64, DbError> {
    conn.execute(
        "INSERT INTO manga (source, url, title, cover_url, description, blob_json, added_at, library)
         VALUES (?1,?2,?3,?4,?5,?6,?7,0)
         ON CONFLICT(source, url) DO UPDATE SET title=excluded.title, cover_url=excluded.cover_url,
         description=excluded.description, blob_json=excluded.blob_json",
        params![m.source, m.url, m.title, m.cover_url, m.description, m.blob_json(), added_at],
    )?;
    let id: i64 = conn.query_row("SELECT id FROM manga WHERE source=?1 AND url=?2",
        params![m.source, m.url], |r| r.get(0))?;
    Ok(id)
}

pub fn get_by_key(conn: &Connection, source: &str, url: &str) -> Result<Option<Manga>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, source, url, title, cover_url, description, blob_json FROM manga WHERE source=?1 AND url=?2")?;
    let mut rows = stmt.query(params![source, url])?;
    match rows.next()? { Some(row) => Ok(Some(row_to_manga(row)?)), None => Ok(None) }
}

pub fn list_library(conn: &Connection) -> Result<Vec<Manga>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, source, url, title, cover_url, description, blob_json FROM manga WHERE library=1 ORDER BY title COLLATE NOCASE")?;
    let rows = stmt.query_map([], row_to_manga)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn set_library_flag(conn: &Connection, id: i64, in_library: bool) -> Result<(), DbError> {
    conn.execute("UPDATE manga SET library=?1 WHERE id=?2",
        params![if in_library {1} else {0}, id])?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM manga WHERE id=?1", params![id])?;
    Ok(())
}
```

Implement the remaining DAOs with the same pattern:
- `chapter_dao`: `replace_for_manga` = `DELETE FROM chapter WHERE manga_id=?1` then bulk INSERT (url, name, number, blob_json); `list_for_manga` restores `Chapter` with `read` from column; `mark_read` UPDATE.
- `category_dao`: plain CRUD on `category`; `assign`/`unassign` INSERT/`DELETE FROM manga_category`; `for_manga` joins `category` + `manga_category`.
- `history_dao`: `upsert` = `INSERT ... ON CONFLICT(manga_id) DO UPDATE SET chapter_index=excluded.chapter_index, page_index=excluded.page_index, updated_at=excluded.updated_at`; `recent` = `SELECT manga_id, chapter_index, page_index, updated_at FROM history ORDER BY updated_at DESC LIMIT ?1`.
- `download_dao`: `upsert` = `INSERT ... ON CONFLICT(manga_id, chapter_url) DO UPDATE SET state=excluded.state`; `update_progress` and `set_state` are UPDATEs; `list`/`list_by_state` SELECT with `DownloadState` decoded from TEXT via `#[serde(rename_all = "lowercase")]` on a local `FromStr`/`serde_json` mapping.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test dao_test`
Expected: 6 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/db/dao/ tests/dao_test.rs
git commit -m "feat(db): 5 DAOs espejo de la capa drift"
```

---

### Task 10: core/net — descarga de imágenes con caché en disco

**Files:**
- Create: `src/core/net/mod.rs`
- Test: `tests/net_test.rs`

**Interfaces:**
- Produces:

```rust
pub struct ImageCache { root: PathBuf, client: reqwest::Client }
impl ImageCache {
    pub fn new() -> Self;   // root = Xdg::cache_root()
    pub async fn get(&self, url: &str, headers: &HashMap<String, String>) -> Result<PathBuf, NetError>;
    pub fn cached_path(&self, url: &str) -> PathBuf;  // root/<sha256(url)>.img
}
```

`NetError` in `error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("HTTP {status}: {url}")]
    Http { status: reqwest::StatusCode, url: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
}
```

- [ ] **Step 1: Write the failing test**

```rust
// tests/net_test.rs
use bakeneko::core::net::ImageCache;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[tokio::test]
async fn get_caches_and_serves_from_disk() {
    let hits = Arc::new(AtomicUsize::new(0));
    let hits2 = hits.clone();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for _ in 0..1 {  // solo sirve UNA petición
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = [0u8; 4096];
                let _ = s.read(&mut buf);
                hits2.fetch_add(1, Ordering::SeqCst);
                let body = b"FAKEIMAGEDATA";
                let resp = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(resp.as_bytes());
                let _ = s.write_all(body);
            }
        }
    });

    temp_env::set_var("XDG_CACHE_HOME", "/tmp/net-test-cache");
    let cache = ImageCache::new();
    let url = format!("http://127.0.0.1:{}/img.jpg", port);

    let p1 = cache.get(&url, &Default::default()).await.unwrap();
    assert!(p1.exists());
    assert_eq!(std::fs::read(&p1).unwrap(), b"FAKEIMAGEDATA");

    // Segunda llamada: desde caché (el server ya no acepta conexiones).
    let p2 = cache.get(&url, &Default::default()).await.unwrap();
    assert_eq!(p1, p2);
    assert_eq!(hits.load(Ordering::SeqCst), 1);
}

#[test]
fn cached_path_is_stable_sha256() {
    temp_env::set_var("XDG_CACHE_HOME", "/tmp/net-test-cache2");
    let cache = ImageCache::new();
    let a = cache.cached_path("http://x/1.jpg");
    let b = cache.cached_path("http://x/1.jpg");
    assert_eq!(a, b);
    assert_ne!(a, cache.cached_path("http://x/2.jpg"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test net_test`
Expected: FAIL.

- [ ] **Step 3: Implement net/mod.rs**

```rust
use crate::core::error::NetError;
use crate::core::xdg::Xdg;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ImageCache { root: PathBuf, client: reqwest::Client }

impl ImageCache {
    pub fn new() -> Self {
        Self {
            root: Xdg::cache_root(),
            client: reqwest::Client::builder().user_agent("bakeneko-rs/0.1").build().unwrap(),
        }
    }

    pub fn cached_path(&self, url: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hex: String = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
        self.root.join(format!("{hex}.img"))
    }

    pub async fn get(&self, url: &str, headers: &HashMap<String, String>) -> Result<PathBuf, NetError> {
        let path = self.cached_path(url);
        if path.exists() { return Ok(path); }
        let mut req = self.client.get(url);
        for (k, v) in headers { req = req.header(k, v); }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(NetError::Http { status: resp.status(), url: url.to_string() });
        }
        let bytes = resp.bytes().await?;
        std::fs::create_dir_all(&self.root)?;
        std::fs::write(&path, bytes)?;
        Ok(path)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test net_test`
Expected: 2 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/error.rs src/core/net/mod.rs tests/net_test.rs
git commit -m "feat(net): ImageCache con sha256(url) en XDG cache"
```

---

### Task 11: app.rs — AppState, Message, update, subscription, view

**Files:**
- Create: `src/app.rs` (replace stub)
- Create: `src/features/mod.rs`, `src/features/shell/mod.rs`
- Modify: `src/main.rs`
- Test: `tests/app_test.rs` (unit tests for the update reducer logic)

**Interfaces:**
- Consumes: everything from Tasks 1-10.
- Produces:

```rust
pub struct AppState {
    pub screen: Screen,
    pub error: Option<String>,
    pub settings: Settings,
    pub sources: Vec<Source>,
    pub daemon_ready: bool,
    pub daemon: Option<Arc<DaemonClient>>,
    pub db: Option<Arc<Mutex<Connection>>>,
    pub cache: Arc<ImageCache>,
}
pub enum Message {
    DaemonStarted(Result<PingReply, DaemonError>),
    SourcesListed(Result<Vec<Source>, DaemonError>),
    CatalogListed(Result<Vec<Manga>, DaemonError>),
    DaemonDied,
    NavigateTo(Screen),
    ErrorDismissed,
}
pub fn update(state: &mut AppState, msg: Message) -> Task<Message>;
pub fn subscription(&self) -> Subscription<Message>;
pub fn view(state: &AppState) -> Element<'_, Message>;
```

The 8 feature screens each define their own `Message` sub-enum, wrapped via `Message::Browse(browse::Message)` etc. To keep the plan tractable: **Task 11 wires only Home + Browse with their navigation; each later screen task (12-16) adds its enum variant + view + `NavigateTo` arm + a `Task::perform` in `update`.**

- [ ] **Step 1: Write the failing reducer test**

```rust
// tests/app_test.rs
use bakeneko::app::{Message, update, AppState};
use bakeneko::core::models::{PingReply, Source};

#[test]
fn daemon_started_sets_ready_and_error_none() {
    let mut s = AppState::default();
    s.daemon_ready = false;
    let _task = update(&mut s, Message::DaemonStarted(Ok(PingReply { version: "1.0.0".into(), java: "21".into() })));
    assert!(s.daemon_ready);
    assert!(s.error.is_none());
}

#[test]
fn daemon_started_error_sets_error() {
    let mut s = AppState::default();
    let _task = update(&mut s, Message::DaemonStarted(Err(bakeneko::core::error::DaemonError::Spawn("boom".into()))));
    assert!(!s.daemon_ready);
    assert!(s.error.is_some());
}

#[test]
fn sources_listed_populates() {
    let mut s = AppState::default();
    let _task = update(&mut s, Message::SourcesListed(Ok(vec![Source { id: "MANGADEX".into(), name: "MangaDex".into() }])));
    assert_eq!(s.sources.len(), 1);
}

#[test]
fn navigate_to_changes_screen() {
    let mut s = AppState::default();
    let _task = update(&mut s, Message::NavigateTo(bakeneko::features::Screen::Browse));
    assert!(matches!(s.screen, bakeneko::features::Screen::Browse));
}
```

Note: `AppState::default()` needs `#[derive(Default)]` with sensible defaults (dark theme, Browse screen, daemon not started). `Screen` must be `#[derive(Default)]` with `#[default] Home`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test app_test`
Expected: FAIL.

- [ ] **Step 3: Implement features/mod.rs (Screen + router)**

```rust
pub mod browse;
pub mod details;
pub mod downloads;
pub mod extensions;
pub mod home;
pub mod library;
pub mod reader;
pub mod settings;
pub mod shell;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Home,
    Browse,
    Details,
    Library,
    Reader,
    Downloads,
    Settings,
    Extensions,
}
```

- [ ] **Step 4: Implement shell/mod.rs (nav rail)**

```rust
use iced::widget::{button, column, horizontal_space, row, text, Container};
use iced::{Element, Length};

use super::Screen;

pub struct Shell;

pub fn view<'a, M: 'a>(screen: &Screen, content: Element<'a, M>) -> Element<'a, M>
where M: From<NavMsg> {
    let items = [
        ("Inicio", Screen::Home),
        ("Explorar", Screen::Browse),
        ("Biblioteca", Screen::Library),
        ("Lector", Screen::Reader),
        ("Descargas", Screen::Downloads),
        ("Ajustes", Screen::Settings),
        ("Extensiones", Screen::Extensions),
    ];
    let rail = column(
        items.into_iter().map(|(label, target)| {
            let btn = button(text(label)).on_press(NavMsg::Navigate(target.clone()));
            if *screen == target { btn } else { btn }
        }).collect(),
    ).spacing(4);
    row![rail, content].into()
}

#[derive(Debug, Clone)]
pub enum NavMsg { Navigate(Screen) }
```

Note: `M: From<NavMsg>` requires the app's global `Message` to impl `From<NavMsg>`. In app.rs:

```rust
impl From<shell::NavMsg> for Message {
    fn from(n: shell::NavMsg) -> Self { match n { shell::NavMsg::Navigate(s) => Message::NavigateTo(s) } }
}
```

Also require `iced::widget::row!`/`column!` macros import (`use iced::widget::{row, column};`).

- [ ] **Step 5: Implement app.rs (state + update + subscription + view)**

```rust
use iced::futures::StreamExt;
use iced::widget::{center, column, text};
use iced::{Element, Length, Subscription, Task};
use std::sync::{Arc, Mutex};

use crate::core::daemon::client::DaemonClient;
use crate::core::daemon::api::MangaSourceApi;
use crate::core::error::DaemonError;
use crate::core::models::{PingReply, Source};
use crate::core::net::ImageCache;
use crate::core::settings::Settings;
use crate::features::shell::{self, NavMsg};
use crate::features::{Screen, browse, home};

#[derive(Debug, Default)]
pub struct AppState {
    pub screen: Screen,
    pub error: Option<String>,
    pub settings: Settings,
    pub sources: Vec<Source>,
    pub daemon_ready: bool,
    pub daemon: Option<Arc<DaemonClient>>,
    pub db: Option<Arc<Mutex<rusqlite::Connection>>>,
    pub cache: Arc<ImageCache>,
    pub home: home::State,
    pub browse: browse::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    DaemonStarted(Result<PingReply, DaemonError>),
    SourcesListed(Result<Vec<Source>, DaemonError>),
    CatalogListed(Result<Vec<bakeneko_core::models::Manga>, DaemonError>),
    DaemonDied,
    NavigateTo(Screen),
    ErrorDismissed,
    Home(home::Message),
    Browse(browse::Message),
}

impl From<NavMsg> for Message {
    fn from(n: NavMsg) -> Self { match n { NavMsg::Navigate(s) => Message::NavigateTo(s) } }
}

pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::DaemonStarted(Ok(_)) => {
            state.daemon_ready = true;
            state.error = None;
            // dispara la carga inicial de fuentes
            let d = state.daemon.clone();
            if let Some(d) = d {
                let d2 = d.clone();
                return Task::perform(async move { d2.list_sources().await },
                    Message::SourcesListed);
            }
            Task::none()
        }
        Message::DaemonStarted(Err(e)) => { state.daemon_ready = false; state.error = Some(e.to_string()); Task::none() }
        Message::SourcesListed(Ok(s)) => { state.sources = s; Task::none() }
        Message::SourcesListed(Err(e)) => { state.error = Some(e.to_string()); Task::none() }
        Message::CatalogListed(Ok(m)) => { state.browse.list = m; Task::none() }
        Message::CatalogListed(Err(e)) => { state.error = Some(e.to_string()); Task::none() }
        Message::DaemonDied => { state.daemon_ready = false; state.error = Some("Daemon cerró el socket".into()); Task::none() }
        Message::NavigateTo(s) => { state.screen = s; Task::none() }
        Message::ErrorDismissed => { state.error = None; Task::none() }
        Message::Home(m) => home::update(state, m),
        Message::Browse(m) => browse::update(state, m),
    }
}

pub fn subscription(state: &AppState) -> Subscription<Message> {
    // Emite Message::DaemonDied si el socket se cierra. Para mantenerlo simple
    // en esta fase, es una subscription vacía; se llena en Task 13.
    Subscription::none()
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let content: Element<Message> = match state.screen {
        Screen::Home => home::view(state),
        Screen::Browse => browse::view(state),
        _ => center(text("Pantalla en construcción")).into(),
    };
    shell::view(&state.screen, content)
}
```

Note: `home::update(state, m)`/`browse::update(state, m)` operate on `&mut AppState` and return `Task<Message>` (owned state is mutated through the single `AppState`; the feature modules expose `State` structs embedded in `AppState`).

- [ ] **Step 6: Implement main.rs**

```rust
mod app;
mod core;
mod features;
mod theme;

use iced::{Application, Settings};
use app::{AppState, Message};

impl Application for App { type Executor = iced::executor::Default; type Message = Message; type Theme = iced::Theme; type Flags = (); }

struct App { state: AppState }

impl Default for App { fn default() -> Self { Self { state: AppState::default() } } }

impl Application for App {
    fn new(_flags: ()) -> (App, Task<Message>) { (App::default(), Task::none()) }
    fn title(&self) -> String { "Bakeneko Reader".into() }
    fn update(&mut self, msg: Message) -> Task<Message> { app::update(&mut self.state, msg) }
    fn view(&self) -> Element<'_, Message> { app::view(&self.state) }
}

fn main() -> iced::Result {
    if let Err(e) = core::xdg::Xdg::ensure_dirs() { eprintln!("XDG dirs: {e}"); }
    App::run(Settings::with_flags(()))
}
```

- [ ] **Step 7: Build + smoke test**

Run: `cargo build`
Expected: compiles. Then `cargo test --test app_test` → 4 PASS.

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/main.rs src/features/ src/theme.rs tests/app_test.rs
git commit -m "feat(ui): AppState + Message + update + view + shell nav rail"
```

---

### Task 12: features/browse + features/home

**Files:**
- Create: `src/features/browse/mod.rs`, `src/features/home/mod.rs`

**Interfaces:**
- Consumes: `MangaSourceApi` (Task 6), `AppState`/`Message` (Task 11).
- Produces:

```rust
pub mod browse {
    #[derive(Debug, Default)] pub struct State { pub source: Option<String>, pub offset: i32, pub query: Option<String>, pub list: Vec<Manga>, pub loading: bool }
    #[derive(Debug, Clone)] pub enum Message { SourceSelected(String), Refresh, QueryChanged(String), More }
    pub fn update(state: &mut crate::app::AppState, msg: Message) -> Task<crate::app::Message>;
    pub fn view(state: &crate::app::AppState) -> Element<'_, crate::app::Message>;
}
pub mod home {
    #[derive(Debug, Default)] pub struct State { pub recent: Vec<Manga> }
    #[derive(Debug, Clone)] pub enum Message { LoadRecent }
    pub fn update(state: &mut crate::app::AppState, msg: Message) -> Task<crate::app::Message>;
    pub fn view(state: &crate::app::AppState) -> Element<'_, crate::app::Message>;
}
```

- [ ] **Step 1: Implement browse/mod.rs**

```rust
use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Element, Task};
use crate::app::{AppState, Message};
use crate::core::daemon::api::MangaSourceApi;

#[derive(Debug, Default)]
pub struct State {
    pub source: Option<String>,
    pub offset: i32,
    pub query: Option<String>,
    pub list: Vec<crate::core::models::Manga>,
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub enum Message { SourceSelected(String), Refresh, QueryChanged(String), More }

pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::SourceSelected(s) => { state.browse.source = Some(s.clone()); state.browse.offset = 0;
            state.browse.list.clear(); state.browse.loading = true;
            let d = state.daemon.clone();
            if let Some(d) = d { Task::perform(async move { d.catalog_list(&s, 0, None).await }, Message::CatalogListed) }
            else { Task::none() } }
        Message::Refresh => {
            let s = state.browse.source.clone().unwrap_or_default();
            let q = state.browse.query.clone();
            state.browse.offset = 0; state.browse.loading = true;
            let d = state.daemon.clone();
            if let Some(d) = d { Task::perform(async move { d.catalog_list(&s, 0, q.as_deref()).await }, Message::CatalogListed) }
            else { Task::none() } }
        Message::QueryChanged(q) => { state.browse.query = Some(q); Task::none() }
        Message::More => {
            state.browse.offset += 20; let s = state.browse.source.clone().unwrap_or_default();
            let off = state.browse.offset;
            let d = state.daemon.clone();
            if let Some(d) = d { Task::perform(async move { d.catalog_list(&s, off, None).await }, Message::CatalogListed) }
            else { Task::none() } }
    }
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let sources = state.sources.clone();
    let source_row = row(
        sources.into_iter().map(|s| {
            let is_sel = Some(s.id.clone()) == state.browse.source;
            let mut b = button(text(if is_sel { s.name.clone() + " ✓" } else { s.name.clone() }));
            if !is_sel { b = b.on_press(Message::Browse(Message::SourceSelected(s.id))); }
            b.into()
        }).collect::<Vec<Element<Message>>>())
        .spacing(8);
    let list = column(state.browse.list.iter().map(|m| {
        button(text(&m.title)).on_press(Message::NavigateTo(crate::features::Screen::Details)).into()
    }).collect()).spacing(4);
    let more = button(text("Más")).on_press(Message::Browse(Message::More));
    column![source_row, scrollable(list), more].spacing(8).into()
}
```

- [ ] **Step 2: Implement home/mod.rs**

```rust
use iced::widget::{button, column, text};
use iced::{Element, Task};
use crate::app::{AppState, Message};
use crate::features::Screen;

#[derive(Debug, Default)]
pub struct State { pub recent: Vec<crate::core::models::Manga> }

#[derive(Debug, Clone)]
pub enum Message { LoadRecent, RecentLoaded(Result<Vec<crate::core::models::Manga>, DbError>) }

pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::LoadRecent => {
            let db = state.db.clone();
            if let Some(db) = db {
                let conn = db.lock().unwrap().clone();
                Task::perform(
                    tokio::task::spawn_blocking(move || {
                        // recent: ids de history ordenados por updated_at, mapeados a Manga
                        let ids = crate::core::db::dao::history_dao::recent(&conn, 10)?;
                        let mut out = vec![];
                        for (id, _, _, _) in ids {
                            if let Some(m) = manga_dao::get_by_id(&conn, id)? { out.push(m); }
                        }
                        Ok(out)
                    }),
                    |r| Message::Home(Message::RecentLoaded(r)),
                )
            } else { Task::none() }
        }
        Message::RecentLoaded(Ok(recent)) => { state.home.recent = recent; Task::none() }
        Message::RecentLoaded(Err(e)) => { state.error = Some(e.to_string()); Task::none() }
    }
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = text("Bakeneko").size(32);
    let recent = state.home.recent.clone();
    let recent_row = column(recent.into_iter().map(|m| {
        button(text(&m.title)).on_press(Message::NavigateTo(Screen::Details)).into()
    }).collect()).spacing(4);
    let lib = state.library.clone();
    let lib_row = column(lib.into_iter().map(|m| {
        button(text(&m.title)).on_press(Message::NavigateTo(Screen::Details)).into()
    }).collect()).spacing(4);
    column![header, text("Continuar leyendo"), recent_row, text("Biblioteca"), lib_row].spacing(8).into()
}
```

Note: this requires `manga_dao::get_by_id(conn, id)` — add it to Task 9's `manga_dao` (same row mapping as `get_by_key`, `WHERE id=?1`).
```

- [ ] **Step 3: Wire the missing `library` field on AppState**

In `app.rs`, add `pub library: Vec<Manga>` to `AppState` and `#[default]` empty. Also add `use crate::features::browse;` etc. as needed.

- [ ] **Step 4: Build**

Run: `cargo build`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add src/features/browse/mod.rs src/features/home/mod.rs src/app.rs
git commit -m "feat(features): browse (fuentes + catálogo) y home (biblioteca)"
```

---

### Task 13: Daemon lifecycle — start en main + socket-watch subscription

**Files:**
- Modify: `src/main.rs`, `src/app.rs`

**Interfaces:**
- Consumes: `DaemonClient` (Task 7).
- Produces: `App` gets a `daemon: Option<Arc<DaemonClient>>`; `update` handles `Message::DaemonStarted` → triggers `SourcesListed`.

- [ ] **Step 1: Modify main.rs to start the daemon before the UI**

```rust
fn main() -> iced::Result {
    if let Err(e) = core::xdg::Xdg::ensure_dirs() { eprintln!("XDG dirs: {e}"); }
    App::run(Settings::with_flags(()))
}

impl Application for App {
    fn new(_flags: ()) -> (App, Task<Message>) {
        let mut daemon = DaemonClient::new();
        let jar = DaemonClient::default_jar_path();
        let state = AppState { daemon_ready: false, ..Default::default() };
        let app = App { state };
        let start = Task::perform(async move {
            daemon.start(Some(&jar.to_string_lossy()), None).await.map(|_| {
                // conecta y dispara ping
                tokio::spawn(async move {
                    let _ = daemon;
                });
                PingReply { version: "1.0.0".into(), java: "".into() }
            })
        }, Message::DaemonStarted);
        (app, start)
    }
}
```

Hmm — this closure captures `daemon` by value but we need it inside `AppState`. **Correction for the real implementation:** keep a single `DaemonClient` owned by `AppState` as `Arc<DaemonClient>`, spawn the start inside `update` via a `Command`:

```rust
pub enum Message { DaemonBooting, DaemonStarted(Result<PingReply, DaemonError>), /* ... */ }
```

In `main.rs::new`, create the client, store it in state, and return `Task::perform(daemon.start_async(jar), Message::DaemonStarted)` where `start_async` is an async method on a cloneable handle. Simpler concrete design (use this):

```rust
// client.rs — add an async constructor for the UI path
impl DaemonClient {
    pub fn spawn_arc(jar: &str) -> Arc<DaemonClient> {
        let c = Arc::new(DaemonClient::new());
        let c2 = c.clone();
        let jar = jar.to_string();
        tokio::spawn(async move { let _ = c2.clone().start_sync(&jar).await; });
        c
    }
}
```

Then `main.rs`:

```rust
fn new(_flags: ()) -> (App, Task<Message>) {
    let jar = DaemonClient::default_jar_path();
    let state = AppState { daemon: Some(DaemonClient::spawn_arc(&jar.to_string_lossy())), daemon_ready: false, ..Default::default() };
    (App { state }, Task::none())
}
```

The background task completes the pending `ping` (sent in `start`) → the reader dispatches → `Task::perform` in `update` for `DaemonStarted` resolves through the `oneshot`. Simpler still and equally valid: make `start()` return `()`, send an explicit ping from `update` after a `DaemonReady` message. Implement whichever is simplest; the contract is `daemon_ready == true` implies `sources.list` was called at least once.

- [ ] **Step 2: Socket-watch subscription (Message::DaemonDied)**

```rust
pub fn subscription(state: &AppState) -> Subscription<Message> {
    if let Some(d) = &state.daemon {
        let d = d.clone();
        Subscription::run_with_id(
            "daemon-socket",
            iced::stream::channel(16, move |mut tx| async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if !d.is_alive() { let _ = tx.send(Message::DaemonDied).await; break; }
                }
            }),
        )
    } else { Subscription::none() }
}
```

Add to client.rs: `pub fn is_alive(&self) -> bool { self.socket.is_some() }`.

- [ ] **Step 3: Build + manual smoke**

Run: `cargo build`
Expected: compiles. Manual: `cargo run` with the daemon JAR built (`cd daemon && ./gradlew shadowJar`) → window opens, nav rail renders, Home shows empty library.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/main.rs src/core/daemon/client.rs
git commit -m "feat(ui): lifecycle del daemon + socket-watch subscription"
```

---

### Task 14: features/library + features/details (con DB)

**Files:**
- Create: `src/features/library/mod.rs`, `src/features/details/mod.rs`
- Modify: `src/app.rs`, `src/features/mod.rs`

**Interfaces:**
- Consumes: `Database`/DAOs (Tasks 8-9), `MangaSourceApi` (Task 6).
- Produces:

```rust
pub mod library {
    #[derive(Debug, Default)] pub struct State { pub list: Vec<Manga>, pub category_filter: Option<i64> }
    #[derive(Debug, Clone)] pub enum Message { Load, CategoryFilter(i64) }
    pub fn update(state: &mut AppState, msg: Message) -> Task<Message>;
    pub fn view(state: &AppState) -> Element<'_, Message>;
}
pub mod details {
    #[derive(Debug, Default)] pub struct State { pub manga: Option<Manga>, pub chapters: Vec<Chapter>, pub loading: bool }
    #[derive(Debug, Clone)] pub enum Message { Load(MangaRef), Fetched(Result<Manga, DaemonError>), ChapterSelected(Chapter), AddToLibrary }
    pub fn update(state: &mut AppState, msg: Message) -> Task<Message>;
    pub fn view(state: &AppState) -> Element<'_, Message>;
}
```

- [ ] **Step 1: Implement library/mod.rs (DB-backed)**

```rust
use iced::widget::{button, column, scrollable, text};
use iced::{Element, Task};
use crate::app::{AppState, Message};
use crate::core::db::dao::manga_dao;
use crate::core::error::DbError;

#[derive(Debug, Default)]
pub struct State { pub list: Vec<crate::core::models::Manga>, pub category_filter: Option<i64> }

#[derive(Debug, Clone)]
pub enum Message { Load }

pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::Load => {
            let db = state.db.clone().unwrap();
            let conn = db.lock().unwrap().clone();
            let (tx, rx) = tokio::sync::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(manga_dao::list_library(&conn));
            });
            Task::perform(async { rx.await.unwrap() }, Message::LibraryLoaded)
        }
    }
}
```

Note: `Message::LibraryLoaded(Result<Vec<Manga>, DbError>)` must be added to the global `Message` in `app.rs`, with an `update` arm `Ok(list) => { state.library = list; Task::none() }`. Because `rusqlite::Connection` is `Send` but not `Sync`, use the DB through the `Arc<Mutex<Connection>>` and run the DAO call inside a `tokio::task::spawn_blocking` (prefer `db_blocking` from Task 8 over manual `std::thread::spawn`).

```rust
pub fn view(state: &AppState) -> Element<'_, Message> {
    let list = state.library.clone();
    column(list.into_iter().map(|m| {
        button(text(&m.title)).on_press(Message::NavigateTo(crate::features::Screen::Details)).into()
    }).collect()).spacing(4).into()
}
```

- [ ] **Step 2: Implement details/mod.rs (daemon details + chapters)**

```rust
use iced::widget::{button, column, scrollable, text};
use iced::{Element, Task};
use crate::app::{AppState, Message};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::error::DaemonError;
use crate::core::models::{Chapter, Manga, MangaRef};

#[derive(Debug, Default)]
pub struct State { pub manga: Option<Manga>, pub chapters: Vec<Chapter>, pub loading: bool }

#[derive(Debug, Clone)]
pub enum Message { Load(MangaRef), Fetched(Result<Manga, DaemonError>), AddToLibrary }

pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::Load(mref) => {
            state.details.loading = true;
            let d = state.daemon.clone();
            let src = mref.source.clone();
            let m = Manga { source: mref.source, url: mref.url, title: mref.title, ..Default::default() };
            if let Some(d) = d {
                Task::perform(async move { d.manga_details(&src, &m).await }, Message::DetailsFetched)
            } else { Task::none() }
        }
        Message::Fetched(Ok(manga)) => {
            state.details.manga = Some(manga.clone());
            state.details.chapters = manga.chapters.clone();
            state.details.loading = false;
            // persiste en DB
            let db = state.db.clone();
            if let Some(db) = db {
                let m2 = manga.clone();
                let conn = db.lock().unwrap().clone();
                let _ = std::thread::spawn(move || { let _ = manga_dao::upsert(&conn, &m2, 0); });
            }
            Task::none()
        }
        Message::Fetched(Err(e)) => { state.error = Some(e.to_string()); state.details.loading = false; Task::none() }
        Message::AddToLibrary => {
            if let Some(m) = &state.details.manga {
                let db = state.db.clone();
                if let Some(db) = db {
                    let m = m.clone();
                    let conn = db.lock().unwrap().clone();
                    let _ = std::thread::spawn(move || { let _ = manga_dao::set_library_flag(&conn, 0, true); });
                }
            }
            Task::none()
        }
    }
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    let Some(m) = &state.details.manga else { return text("Cargando…").into() };
    let header = column![text(&m.title).size(28), if let Some(d) = &m.description { text(d) } else { text("") }];
    let chapters = column(state.details.chapters.iter().map(|c| {
        button(text(&c.title)).into()
    }).collect()).spacing(4);
    column![header, button(text("Agregar a biblioteca")).on_press(Message::Details(Message::AddToLibrary)), scrollable(chapters)].spacing(8).into()
}
```

Note: `Message::DetailsFetched` and `Message::Details(…)` wrappers must exist on the global `Message`; `manga_dao` import at top of `details/mod.rs`. `Manga::default()` requires `impl Default for Manga`.

- [ ] **Step 3: Wire navigation — Details entry**

In `browse::view` and `library::view`, replace the `NavigateTo(Screen::Details)` placeholder with an actual load: send `Message::Details(Message::Load(MangaRef { .. }))` for the tapped manga.

- [ ] **Step 4: Build + manual smoke**

Run: `cargo build`
Expected: compiles. Manual: browse to a source → tap a manga → details loads with description + chapters; Add to library persists.

- [ ] **Step 5: Commit**

```bash
git add src/features/library/mod.rs src/features/details/mod.rs src/app.rs src/features/mod.rs
git commit -m "feat(features): library (DB) + details (daemon details + capítulos)"
```

---

### Task 15: features/reader — visor de páginas

**Files:**
- Create: `src/features/reader/mod.rs`
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `MangaSourceApi` (Task 6), `ImageCache` (Task 10).
- Produces:

```rust
pub mod reader {
    #[derive(Debug, Default)] pub struct State { pub chapter: Option<Chapter>, pub pages: Vec<Page>, pub current: usize, pub loading: bool }
    #[derive(Debug, Clone)] pub enum Message { Load(Chapter), PagesFetched(Result<Vec<Page>, DaemonError>), Prev, Next, PageChanged }
    pub fn update(state: &mut AppState, msg: Message) -> Task<Message>;
    pub fn view(state: &AppState) -> Element<'_, Message>;
}
```

Note: requires `manga_dao::get_id_by_key(&Connection, source, url) -> Result<i64, DbError>` (add to Task 9) and a `now_millis()` helper: `std::time::SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64`. Prev/Next also emit `Message::PageChanged` after updating `current`.

- [ ] **Step 1: Implement reader/mod.rs**

```rust
use iced::widget::{button, column, horizontal_space, image, row, scrollable, text};
use iced::{Element, Task};
use crate::app::{AppState, Message};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::error::DaemonError;
use crate::core::models::{Chapter, Page};

#[derive(Debug, Default)]
pub struct State { pub chapter: Option<Chapter>, pub pages: Vec<Page>, pub current: usize, pub loading: bool }

#[derive(Debug, Clone)]
pub enum Message { Load(Chapter), PagesFetched(Result<Vec<Page>, DaemonError>), Prev, Next }

pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::Load(ch) => {
            state.reader.loading = true;
            // registra historial + marca leído al abrir el capítulo
            let db = state.db.clone();
            if let Some(db) = db {
                let conn = db.lock().unwrap().clone();
                let src = ch.source.clone();
                let url = ch.url.clone();
                let _ = std::thread::spawn(move || {
                    if let Ok(mid) = manga_dao::get_id_by_key(&conn, &src, &url) {
                        let _ = history_dao::upsert(&conn, mid, 0, 0, now_millis());
                    }
                });
            }
            let d = state.daemon.clone();
            let src = ch.source.clone();
            if let Some(d) = d {
                Task::perform(async move { d.chapter_pages(&src, &ch).await }, Message::ReaderPagesFetched)
            } else { Task::none() }
        }
        Message::PagesFetched(Ok(pages)) => { state.reader.pages = pages; state.reader.current = 0; state.reader.loading = false; Task::none() }
        Message::PagesFetched(Err(e)) => { state.error = Some(e.to_string()); state.reader.loading = false; Task::none() }
        Message::Prev => { state.reader.current = state.reader.current.saturating_sub(1); Task::none() }
        Message::Next => { if state.reader.current + 1 < state.reader.pages.len() { state.reader.current += 1; } Task::none() }
        Message::PageChanged => {
            // actualiza history.page_index al cambiar de página
            let db = state.db.clone();
            if let Some(db) = db {
                let conn = db.lock().unwrap().clone();
                let src = state.reader.chapter.as_ref().map(|c| c.source.clone()).unwrap_or_default();
                let url = state.reader.chapter.as_ref().map(|c| c.url.clone()).unwrap_or_default();
                let idx = state.reader.current as i32;
                let _ = std::thread::spawn(move || {
                    if let Ok(mid) = manga_dao::get_id_by_key(&conn, &src, &url) {
                        let _ = history_dao::upsert(&conn, mid, idx, idx, now_millis());
                    }
                });
            }
            Task::none()
        }
    }
}

pub fn view(state: &AppState) -> Element<'_, Message> {
    if state.reader.loading || state.reader.pages.is_empty() { return text("Cargando…").into(); }
    let page = &state.reader.pages[state.reader.current];
    let img = image(page.url.clone()).width(iced::Length::Fill);  // ImageCache path no disponible aún: se reemplaza en Task 16
    let nav = row![
        button(text("‹")).on_press(Message::Reader(Message::Prev)),
        horizontal_space(),
        text(format!("{} / {}", state.reader.current + 1, state.reader.pages.len())),
        horizontal_space(),
        button(text("›")).on_press(Message::Reader(Message::Next)),
    ];
    column![scrollable(img), nav].into()
}
```

Note: the `image()` widget needs a real path/handle; replace `page.url` with the cached path via `ImageCache` once Task 16 lands. Add `Message::ReaderPagesFetched` + `Message::Reader(…)` to global `Message`.

- [ ] **Step 2: Wire from details**

In `details::view`, each chapter button gains `.on_press(Message::Reader(Message::Load(c.clone())))` and `Message::NavigateTo(Screen::Reader)`.

- [ ] **Step 3: Build + smoke**

Run: `cargo build`
Expected: compiles. Manual: open a manga → tap chapter → reader shows pages with ‹ › nav.

- [ ] **Step 4: Commit**

```bash
git add src/features/reader/mod.rs src/app.rs
git commit -m "feat(features): reader con páginas + navegación ‹ ›"
```

---

### Task 16: Prefetch e imágenes reales en el reader

**Files:**
- Modify: `src/features/reader/mod.rs`, `src/app.rs`

**Interfaces:**
- Consumes: `ImageCache` (Task 10), `MangaSourceApi::page_url`/`source_headers` (Task 6).

- [ ] **Step 1: Replace placeholder image with cached fetch**

```rust
// en reader::view — para la página actual y la siguiente
let url = page.url.clone();
let handle = state.cache.get_handle(&url, &headers).await; // CacheHandle guarda PathBuf
```

Add to `net/mod.rs`:

```rust
impl ImageCache {
    pub async fn get_handle(&self, url: &str, headers: &HashMap<String,String>) -> Option<iced::widget::image::Handle> {
        let path = self.get(url, headers).await.ok()?;
        let bytes = std::fs::read(path).ok()?;
        iced::widget::image::Handle::from_bytes(bytes).into()
    }
}
```

- [ ] **Step 2: Prefetch prev/next pages on page change**

In `update`'s `Message::Next`/`Prev`, spawn a background prefetch of `pages[current±1]` via `ImageCache` (fire-and-forget task) so the next page is already on disk when rendered.

- [ ] **Step 3: Build + smoke**

Run: `cargo build`
Expected: compiles. Manual: reader shows actual manga page images, prefetch keeps nav snappy.

- [ ] **Step 4: Commit**

```bash
git add src/features/reader/mod.rs src/core/net/mod.rs src/app.rs
git commit -m "feat(reader): imágenes reales con caché + prefetch de páginas vecinas"
```

---

### Task 17: core/downloads — DownloadManager

**Files:**
- Create: `src/core/downloads/mod.rs`
- Modify: `src/core/error.rs`
- Test: `tests/download_manager_test.rs`

**Interfaces:**
- Consumes: `MangaSourceApi` (Task 6), `ImageCache` (Task 10), DAOs (Task 9).
- Produces:

```rust
pub enum DownloadEvent { Queued(i64, String), Progress { manga_id: i64, chapter_url: String, done: i32, total: i32 }, Done(i64, String), Errored(i64, String, String) }

pub struct DownloadManager { /* state: Arc<Mutex<Inner>> */ }
pub struct Inner { db: Arc<Mutex<Connection>>, daemon: Arc<dyn MangaSourceApi>, cache: Arc<ImageCache>, root: PathBuf, concurrency: usize }

impl DownloadManager {
    pub fn new(db: Arc<Mutex<Connection>>, daemon: Arc<dyn MangaSourceApi>, cache: Arc<ImageCache>, concurrency: usize) -> Self;
    pub fn enqueue(&self, manga: &Manga, chapter: &Chapter) -> Result<(), DbError>;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<DownloadEvent>;
    pub fn poll_once(&self) -> Result<(), DbError>;   // toma un lote de queued, los descarga
}
```

Algorithm (mirrors Dart `download_manager.dart`):
1. `enqueue`: `download_dao::upsert(state=Queued)`.
2. `poll_once` (called from a tokio task loop, gated by `concurrency`):
   - `download_dao::list_by_state(Queued)` → take up to `concurrency` (minus in-flight).
   - For each: `download_dao::set_state(Downloading)`; `daemon.chapter_pages`; for each page: `daemon.page_url`, `cache.get(page_url, headers)`, `download_dao::update_progress(done,total)`, emit `DownloadEvent::Progress`; finally `download_dao::set_state(Done)`, emit `Done`.
   - On error: `set_state(Error)`, emit `Errored`.

`DownloadError` in `error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("DB error: {0}")]
    Db(#[from] DbError),
    #[error("daemon error: {0}")]
    Daemon(#[from] DaemonError),
    #[error("network error: {0}")]
    Net(#[from] NetError),
}
```

- [ ] **Step 1: Write the failing test**

```rust
// tests/download_manager_test.rs
use bakeneko::core::daemon::api::MangaSourceApi;
use bakeneko::core::db::Database;
use bakeneko::core::db::dao::{download_dao, manga_dao};
use bakeneko::core::downloads::{DownloadEvent, DownloadManager};
use bakeneko::core::models::{Chapter, DownloadState, Manga, Page, PingReply, Source};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

struct FakeDaemon;
#[async_trait]
impl MangaSourceApi for FakeDaemon {
    async fn ping(&self) -> Result<PingReply, bakeneko::core::error::DaemonError> { Ok(PingReply { version: "1".into(), java: "21".into() }) }
    async fn list_sources(&self) -> Result<Vec<Source>, bakeneko::core::error::DaemonError> { Ok(vec![]) }
    async fn catalog_list(&self, _s: &str, _o: i32, _q: Option<&str>) -> Result<Vec<Manga>, bakeneko::core::error::DaemonError> { Ok(vec![]) }
    async fn manga_details(&self, _s: &str, m: &Manga) -> Result<Manga, bakeneko::core::error::DaemonError> { Ok(m.clone()) }
    async fn chapter_pages(&self, _s: &str, _c: &Chapter) -> Result<Vec<Page>, bakeneko::core::error::DaemonError> {
        Ok(vec![Page { source: "S".into(), url: "http://127.0.0.1/1.jpg".into(), preview: None }])
    }
    async fn page_url(&self, _s: &str, _p: &Page) -> Result<String, bakeneko::core::error::DaemonError> { Ok("http://127.0.0.1/1.jpg".into()) }
    async fn source_headers(&self, _s: &str) -> Result<HashMap<String, String>, bakeneko::core::error::DaemonError> { Ok(HashMap::new()) }
}

fn sample_manga() -> Manga { Manga { source: "MANGADEX".into(), url: "/m1".into(), title: "T".into(), ..Default::default() } }

#[test]
fn enqueue_then_poll_marks_done() {
    let db = Database::open(None).unwrap(); db.migrate().unwrap();
    let conn = db.connection();
    let cache = Arc::new(bakeneko::core::net::ImageCache::new());
    let daemon: Arc<dyn MangaSourceApi> = Arc::new(FakeDaemon);
    let mgr = DownloadManager::new(conn.clone(), daemon, cache, 2);
    let m = sample_manga();
    let id = manga_dao::upsert(&conn.lock().unwrap(), &m, 0).unwrap();
    let ch = Chapter { source: "MANGADEX".into(), url: "/c1".into(), title: "C1".into(), ..Default::default() };
    mgr.enqueue(&m, &ch).unwrap();
    assert_eq!(download_dao::list_by_state(&conn.lock().unwrap(), DownloadState::Queued).unwrap().len(), 1);
    mgr.poll_once().unwrap();
    assert_eq!(download_dao::list_by_state(&conn.lock().unwrap(), DownloadState::Done).unwrap().len(), 1);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test download_manager_test`
Expected: FAIL.

- [ ] **Step 3: Implement downloads/mod.rs**

```rust
use crate::core::daemon::api::MangaSourceApi;
use crate::core::db::dao::download_dao;
use crate::core::error::{DbError, DownloadError};
use crate::core::models::{Chapter, DownloadState, Manga};
use crate::core::net::ImageCache;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Queued(i64, String),
    Progress { manga_id: i64, chapter_url: String, done: i32, total: i32 },
    Done(i64, String),
    Errored(i64, String, String),
}

pub struct Inner {
    pub db: Arc<Mutex<Connection>>,
    pub daemon: Arc<dyn MangaSourceApi>,
    pub cache: Arc<ImageCache>,
    pub root: PathBuf,
    pub concurrency: usize,
}

pub struct DownloadManager { inner: Arc<Inner>, tx: broadcast::Sender<DownloadEvent> }

impl DownloadManager {
    pub fn new(db: Arc<Mutex<Connection>>, daemon: Arc<dyn MangaSourceApi>, cache: Arc<ImageCache>, concurrency: usize) -> Self {
        let (tx, _) = broadcast::channel(64);
        let root = crate::core::xdg::Xdg::downloads_root();
        Self { inner: Arc::new(Inner { db, daemon, cache, root, concurrency }), tx }
    }

    pub fn enqueue(&self, m: &Manga, ch: &Chapter) -> Result<(), DbError> {
        let conn = self.inner.db.lock().unwrap();
        let id = manga_dao::upsert(&conn, m, 0)?;
        download_dao::upsert(&conn, id, &ch.url, DownloadState::Queued)?;
        let _ = self.tx.send(DownloadEvent::Queued(id, ch.url.clone()));
        Ok(())
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DownloadEvent> { self.tx.subscribe() }

    pub fn poll_once(&self) -> Result<(), DownloadError> {
        let inner = self.inner.clone();
        let conn = inner.db.lock().unwrap();
        let jobs = download_dao::list_by_state(&conn, DownloadState::Queued)?;
        let batch: Vec<_> = jobs.into_iter().take(inner.concurrency).collect();
        drop(conn);
        for job in batch {
            self.run_job(&inner, job.manga_id, &job.chapter_url)?;
        }
        Ok(())
    }

    fn run_job(&self, inner: &Inner, manga_id: i64, chapter_url: &str) -> Result<(), DownloadError> {
        let conn = inner.db.lock().unwrap();
        download_dao::set_state(&conn, manga_id, chapter_url, DownloadState::Downloading)?;
        // capítulo desde DB (blob) para round-trip al daemon
        let chapters = crate::core::db::dao::chapter_dao::list_for_manga(&conn, manga_id)?;
        let ch = chapters.into_iter().find(|c| c.url == chapter_url)
            .ok_or_else(|| DbError::Sql(rusqlite::Error::QueryReturnedNoRows))?;
        let source = ch.source.clone();
        drop(conn);

        let rt = tokio::runtime::Handle::current();
        let pages = rt.block_on(inner.daemon.chapter_pages(&source, &ch))?;
        let headers = rt.block_on(inner.daemon.source_headers(&source)).unwrap_or_default();
        let total = pages.len() as i32;
        let mut done = 0;
        for p in &pages {
            let url = rt.block_on(inner.daemon.page_url(&source, p))?;
            let _ = rt.block_on(inner.cache.get(&url, &headers))?;
            done += 1;
            let conn = inner.db.lock().unwrap();
            download_dao::update_progress(&conn, manga_id, chapter_url, done, total)?;
            let _ = self.tx.send(DownloadEvent::Progress { manga_id, chapter_url: chapter_url.to_string(), done, total });
        }
        let conn = inner.db.lock().unwrap();
        download_dao::set_state(&conn, manga_id, chapter_url, DownloadState::Done)?;
        let _ = self.tx.send(DownloadEvent::Done(manga_id, chapter_url.to_string()));
        Ok(())
    }
}
```

Note: `manga_dao` is needed for `enqueue` (ensure manga row exists). The manager task-loop lives in `app.rs` (`subscription`) — see Task 18.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test download_manager_test`
Expected: 1 PASS.

- [ ] **Step 5: Commit**

```bash
git add src/core/downloads/mod.rs src/core/error.rs tests/download_manager_test.rs
git commit -m "feat(downloads): DownloadManager con cola persistida + eventos"
```

---

### Task 18: Downloads subscription + screen + settings/extensions screens

**Files:**
- Create: `src/features/downloads/mod.rs`, `src/features/settings/mod.rs`, `src/features/extensions/mod.rs`
- Modify: `src/app.rs`

**Interfaces:**
- Consumes: `DownloadManager` (Task 17), `Settings` (Task 3).
- Produces: `Message::DownloadEvent(DownloadEvent)` handled in `update` → updates a `downloads` state vector; `Message::Download(…)` for the screen.

- [ ] **Step 1: App-level subscription for downloads**

```rust
pub fn subscription(state: &AppState) -> Subscription<Message> {
    let mut subs = vec![ /* daemon watch from Task 13 */ ];
    if let Some(mgr) = &state.downloads {
        let mut rx = mgr.subscribe();
        subs.push(Subscription::run_with_id("downloads", iced::stream::channel(64, move |mut tx| async move {
            while let Ok(ev) = rx.recv().await { if tx.send(Message::DownloadEvent(ev)).await.is_err() { break; } }
        })));
    }
    Subscription::batch(subs)
}
```

- [ ] **Step 2: downloads/mod.rs screen**

```rust
#[derive(Debug, Default)]
pub struct State { pub entries: Vec<DownloadEntry> }
#[derive(Debug, Clone)]
pub enum Message { Load }
pub fn update(state: &mut AppState, msg: Message) -> Task<Message> { /* list_by_state(all) → Message::DownloadsLoaded */ }
pub fn view(state: &AppState) -> Element<'_, Message> {
    // tabla: título del capítulo, state (Done/Error/Downloading), barra de progreso done/total
}
```

- [ ] **Step 3: settings/mod.rs screen**

```rust
#[derive(Debug, Clone)]
pub enum Message { ThemeChanged(String), AccentChanged(String), ConcurrencyChanged(u32), LibraryViewChanged(String) }
// update: muta state.settings y llama settings::save(&state.settings)
// view: pickers para theme ("dark"/"light"/"system"), accent hex, download_concurrency, library_view
```

- [ ] **Step 4: extensions/mod.rs screen**

```rust
// view: lista estática de state.sources (fuentes del daemon). Sin gestión real.
```

- [ ] **Step 5: Build + smoke**

Run: `cargo build`
Expected: compiles. Manual: desde details, botón descargar enqueue un capítulo; la pantalla Downloads muestra progreso; settings persiste al reiniciar.

- [ ] **Step 6: Commit**

```bash
git add src/features/downloads/mod.rs src/features/settings/mod.rs src/features/extensions/mod.rs src/app.rs
git commit -m "feat(features): downloads (cola) + settings + extensions"
```

---

### Task 19: theme.rs + polish + AppImage

**Files:**
- Create: `src/theme.rs`
- Modify: `src/main.rs`, `make_universal.sh`, `shell.nix`

**Interfaces:**
- Produces: `pub fn iced_theme(settings: &Settings) -> iced::Theme;` mapping theme string → Iced theme; accent used for `Theme::custom`.

- [ ] **Step 1: Implement theme.rs**

```rust
use crate::core::settings::Settings;

pub fn iced_theme(settings: &Settings) -> iced::Theme {
    match settings.theme.as_str() {
        "light" => iced::Theme::Light,
        "system" => iced::Theme::Dark,   // placeholder: detect system
        _ => iced::Theme::Dark,
    }
}
```

Wire into `Application::theme(&self) -> iced::Theme { theme::iced_theme(&self.state.settings) }`.

- [ ] **Step 2: Update make_universal.sh** (mirror the original but without Flutter)

```bash
#!/usr/bin/env bash
set -euo pipefail
# Bakeneko-Universal: binario Rust + JAR daemon + JRE. Sin Flutter.
APP_VERSION="${1:-0.1.0}"
APPIMAGE_DIR="AppDir"
rm -rf "$APPIMAGE_DIR" && mkdir -p "$APPIMAGE_DIR/usr/bin" "$APPIMAGE_DIR/usr/lib" "$APPIMAGE_DIR/usr/jre"

cargo build --release
cp target/release/bakeneko "$APPIMAGE_DIR/usr/bin/"
cp daemon/build/libs/bakeneko-daemon.jar "$APPIMAGE_DIR/usr/bin/"
cp -r "$JRE_HOME" "$APPIMAGE_DIR/usr/jre/"   # o: cp -r $(dirname $(readlink -f $(which java)))/.. "$APPIMAGE_DIR/usr/jre/"

# AppRun wrapper
cat > "$APPIMAGE_DIR/AppRun" <<'EOF'
#!/bin/sh
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
export JAVA_HOME="$HERE/usr/jre"
exec "$HERE/usr/bin/bakeneko"
EOF
chmod +x "$APPIMAGE_DIR/AppRun"

# appimagetool (descargar si falta)
if [ ! -f appimagetool-x86_64.AppImage ]; then
  wget -q https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x appimagetool-x86_64.AppImage
fi
ARCH=x86_64 ./appimagetool-x86_64.AppImage "$APPIMAGE_DIR" "Bakeneko-Universal-v${APP_VERSION}.AppImage"
```

Note: the daemon JAR resolves relative to `current_exe()` — in the AppDir layout it's next to the binary (`usr/bin/`), so `DaemonClient::default_jar_path()` finds it via the `exec_dir.join("bakeneko-daemon.jar")` candidate. The bundled JRE is found via the `<exec>/jre/bin/java` candidate in `resolve_java`.

- [ ] **Step 3: Update shell.nix** — drop Flutter, keep Rust + JDK + GTK deps for Iced:

```nix
{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = with pkgs; [
    rustc cargo rustfmt clippy
    jdk21            # para compilar el daemon JVM
    pkg-config
    gtk3             # backend de Iced (si se usa tiny-skia no hace falta)
  ];
}
```

- [ ] **Step 4: Build + verify**

Run: `cargo build --release && ./make_universal.sh`
Expected: produces `Bakeneko-Universal-v0.1.0.AppImage`; run it on a clean machine → app opens, daemon spawns from bundled JRE.

- [ ] **Step 5: Commit**

```bash
git add src/theme.rs src/main.rs make_universal.sh shell.nix
git commit -m "feat(build): theme + make_universal.sh (Rust+JAR+JRE) + shell.nix sin Flutter"
```

---
