# Bakeneko → Rust: diseño del port

**Fecha:** 2026-08-10
**Autor:** crow (con brainstorming asistido)
**Referencia original:** https://github.com/CrowRei34/Bakeneko (Flutter/Dart + Kotlin JVM)

## 1. Visión y alcance

Reescribir en Rust **todo salvo el daemon JVM**. El binario Rust spawneará el
mismo `java -jar bakeneko-daemon.jar` y hablará el mismo JSON-RPC 2.0
line-delimited sobre Unix Domain Socket. El daemon Kotlin y la librería
`futon-parsers` no se modifican: protocolo y DTOs **byte-compatibles**. El
app Rust decodifica los DTO y, cuando hace round-trip, los reenvía como blob
opaco — exactamente como hace el Dart actual.

### Se porta (Iced + tokio + rusqlite + reqwest + image)

- `core/daemon/` — `DaemonClient`, `rpc` (encode/decode frames), spawn de `java -jar`.
- `core/db/` — schema idéntica, migraciones `PRAGMA user_version`, DAOs por tabla.
- `core/xdg.rs` — mismas rutas XDG + lectura de `/proc/self/status` para uid.
- `core/settings.rs` — `settings.json` con `serde_json` (formato compatible).
- `core/downloads/` — download manager con cola persistida y progreso.
- `core/models.rs` — `Manga`, `Chapter`, `Page`, `Source`, `DownloadEntry`,
  `Category`, `HistoryEntry`, `MangaRef`.
- `core/net/` — cliente HTTP para imágenes con caché en disco.
- `features/*` — pantallas Iced (home, browse, details, library, reader,
  downloads, settings, extensions).

### No se toca

- `daemon/` (Kotlin). Mismo JAR, mismo RPC, mismos DTO.

### Ganancia del port

Desaparece Flutter + Dart + libepoxy + toda la toolchain Flutter en
`shell.nix`. El AppImage final empaqueta: binario Rust + JAR + JRE. Mucho
más liviano.

## 2. Layout del crate

Single-crate binario con módulos espejo de la estructura Dart:

```
bakeneko-rs/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry: XDG ensure_dirs, iced app run
│   ├── app.rs                  # struct App (Model), enum Message, update, subscription, view
│   ├── core/
│   │   ├── mod.rs
│   │   ├── xdg.rs              # Rutas XDG, lectura uid vía /proc
│   │   ├── settings.rs        # Settings + serde_json (settings.json)
│   │   ├── models.rs          # Manga/Chapter/Page/Source/...
│   │   ├── daemon/
│   │   │   ├── mod.rs
│   │   │   ├── client.rs     # DaemonClient: spawn java, connect socket, send/recv
│   │   │   └── rpc.rs        # RpcRequest/RpcResponse/RpcError/RpcCodes
│   │   ├── db/
│   │   │   ├── mod.rs        # Arc<Mutex<Connection>>, migraciones user_version
│   │   │   ├── schema.rs     # const SCHEMA_SQL: &str (idéntico al Dart)
│   │   │   └── dao/         # manga_dao, chapter_dao, category_dao, history_dao, download_dao
│   │   ├── downloads/
│   │   │   └── mod.rs       # DownloadManager: cola, estados, progreso
│   │   └── net/
│   │       └── mod.rs       # reqwest + caché de imágenes
│   ├── theme.rs              # Tema Iced (oscuro/claro, colores, tipografía)
│   └── features/
│       ├── mod.rs            # Router de pantallas + enum Screen
│       ├── shell/            # Top-level layout (nav rail + content)
│       ├── home/
│       ├── browse/
│       ├── details/
│       ├── library/
│       ├── reader/
│       ├── downloads/
│       ├── settings/
│       └── extensions/
└── tests/                    # Tests de integración espejo de app/test/
```

### Dependencias (Cargo.toml)

| Crate | Uso |
|---|---|
| `iced` (features `tokio`, `image`, `svg`, `advanced`) | UI + runtime |
| `tokio` (features `rt`, `process`, `io-util`, `net`, `sync`, `time`) | Async runtime |
| `rusqlite` (features `bundled`) | SQLite sin dep del sistema |
| `serde` + `serde_json` | Serialización DTO/settings |
| `reqwest` (features `json`, `gzip`) | HTTP imágenes/API |
| `image` | Decodifica JPEG/PNG/WebP para Iced |
| `thiserror` | Errores tipados |
| `anyhow` | Errores en binarios/tests |
| `sha2` | Hashes para paths de descarga |
| `nix` (feature `user`) | `getuid()` fallback robusto |

El módulo `daemon/` Rust es **cliente puro** — no linka nada JVM. El único
binario externo que invoca es `java -jar`.

## 3. Arquitectura runtime y ciclo IPC

### Modelo de procesos (sin cambios respecto al original)

```
┌─────────────────────────────┐
│   Bakeneko (Rust + Iced)    │   UI, estado, lector, DB, descargas
│   tokio runtime             │
│   DaemonClient              │
│   call("chapter.pages") ────┼──► Unix Domain Socket
│         │  ▲                │     $XDG_RUNTIME_DIR/bakeneko/daemon.sock
│         │  │  JSON-RPC \n   │
│         ▼  │                │
│   spawn `java -jar ...`     │
└─────────┼───────────────────┘
          ▼
   Daemon JVM (Kotlin) — sin cambios
```

### Ciclo de vida del `DaemonClient` (espejo del Dart)

1. `start(jar_path?, java_path?)`:
   - Resuelve JAR: `defaultJarPath()` busca en `current_exe().dir()`
     (`bakeneko-daemon.jar`, `lib/bakeneko-daemon.jar`) y sube hasta 8
     niveles buscando `daemon/build/libs/bakeneko-daemon.jar` para dev.
   - Resuelve `java`: JRE bundled (`<exec>/jre/bin/java`) >
     `JAVA_HOME/bin/java` > `java` del PATH.
   - Borra socket viejo (`fs::remove_file`) para que el daemon pueda bind.
   - `tokio::process::Command::new(java).arg("-jar").arg(jar)` con
     `stderr` drenado por task dedicada.
   - Poll de conexión al socket con backoff 100ms hasta 15s de deadline.
     Si no conecta: `stop()` + `DaemonError`.
2. `call(method, params) -> Result<Value>`:
   - Genera `id` incremental (`AtomicU64`).
   - Encola `(id, oneshot::Sender)` en `pending: HashMap<u64, oneshot::Sender<Value>>`.
   - Escribe `RpcRequest::encode() + "\n"` al socket.
   - Devuelve `oneshot::Receiver<Value>` awaited como `Task::perform` →
     `Message::XxxResult`.
3. **Lector del socket** = `Subscription` Iced (o task perpetua tokio):
   lee línea a línea, decodifica `RpcResponse`, busca `pending[id]`,
   completa el `oneshot` con `result` o envía `Err(RpcException)`.
4. `stop()`: cierra socket, SIGTERM, espera exit ≤2s, si no SIGKILL.

### Concurrencia

Única conexión, una pending request por id. El daemon contesta en orden
(no pipelining) — mismo supuesto del Dart.

### Cierre limpio

SIGINT/SIGTERM → `stop()` → mata java, borra socket. Hook vía
`tokio::signal` o Iced `subscription::run_with_id`.

## 4. RPC, modelos y DB

### RPC (`core/daemon/rpc.rs`) — byte-compatible con el Dart

```rust
pub struct RpcRequest { pub id: u64, pub method: String, pub params: Option<serde_json::Value> }
impl RpcRequest { pub fn encode(&self) -> String { /* json con jsonrpc:"2.0" */ } }

pub struct RpcResponse { pub id: Option<u64>, pub result: Option<Value>, pub error: Option<RpcErr> }
pub struct RpcErr { pub code: i32, pub message: String }

#[derive(Debug, thiserror::Error)] pub enum RpcException { ... }

pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}
```

`RpcResponse::decode(line: &str) -> Result<RpcResponse>` usando `serde_json`.

### DaemonClient — API tipada (los 7 métodos)

```rust
impl DaemonClient {
    pub async fn ping(&self) -> Result<PingReply>;
    pub async fn list_sources(&self) -> Result<Vec<Source>>;
    pub async fn catalog_list(&self, source: &str, offset: i32, query: Option<&str>) -> Result<Vec<Manga>>;
    pub async fn manga_details(&self, source: &str, manga: &Manga) -> Result<Manga>;
    pub async fn chapter_pages(&self, source: &str, chapter: &Chapter) -> Result<Vec<Page>>;
    pub async fn page_url(&self, source: &str, page: &Page) -> Result<String>;
    pub async fn source_headers(&self, source: &str) -> Result<HashMap<String, String>>;
}
```

Internamente cada uno llama a `self.call(method, json!{...})` y decodifica
vía `serde_json::from_value`. El **blob se preserva**:
`Manga::blob: serde_json::Map<String, Value>` que se reenvía intacto al
daemon en `manga.details`, `chapter.pages`, `page.url` — mismo truco que
en Dart (`blob = j`).

### Modelos (`core/models.rs`)

Structs `#[derive(Serialize, Deserialize, Clone)]` con `#[serde(default)]`
en opcionales:

```rust
pub struct Manga {
    pub source: String, pub url: String, pub title: String,
    pub publicUrl: Option<String>, pub rating: f32, pub isNsfw: bool,
    pub coverUrl: Option<String>, pub largeCoverUrl: Option<String>,
    pub description: Option<String>, pub authors: Vec<String>,
    pub state: Option<String>, pub chapters: Vec<Chapter>,
    #[serde(skip)] pub blob: serde_json::Map<String, Value>,
}
impl Manga { pub fn key(&self) -> String { format!("{}|{}", self.source, self.url) } }

pub struct Chapter { /* source, url, title, number, volume, scanlator, uploadDate, branch, blob, read */ }
pub struct Page { pub source: String, pub url: String, pub preview: Option<String> }
pub struct Source { pub id: String, pub name: String }
pub struct PingReply { pub version: String, pub java: String }  // de `ping`
pub enum DownloadState { Idle, Queued, Downloading, Done, Error }  // serde rename_all lowercase
pub struct DownloadEntry { pub manga_id: i64, pub chapter_url: String, pub state: DownloadState, pub total_pages: i32, pub done_pages: i32 }
pub struct Category { pub id: Option<i64>, pub name: String, pub color: String, pub auto_download: bool, pub created_at: i64 }
pub struct HistoryEntry { pub manga: Manga, pub chapter_index: i32, pub page_index: i32, pub updated_at: i64 }
pub struct MangaRef { pub source: String, pub url: String, pub title: String }
```

`blob_json` persistido en DB = `serde_json::to_string(&manga.blob)`
(corresponde a `encodeBlob()` del Dart).

**Round-trip del blob:** cuando la app llama a `manga.details` /
`chapter.pages` / `page.url`, envía el `blob` opaco (el JSON recibido
originalmente del daemon) en lugar de re-serializar el modelo. Si `blob`
está vacío (p.ej. Manga construido localmente), se envía el modelo
serializado. El daemon reconstruye su `Manga`/`MangaChapter`/`MangaPage`
desde ese JSON (ver `MangaDto.toModel()` en Kotlin, que solo usa los
campos `source`, `url`, `title`, `publicUrl`, `rating`, etc.).

### DB (`core/db/`)

- **Schema SQL idéntica** a `schema.dart` (texto crudo en `const SCHEMA_SQL`).
- **Migraciones**: `PRAGMA user_version` + bloques `IF v < N { exec(...); v = N }`.
  Compatible con DBs creadas por la versión Dart.
- **Conexión**: `Arc<Mutex<Connection>>` (rusqlite no es Sync entre hilos).
  Las queries se ejecutan vía `tokio::task::spawn_blocking` -> `Task::perform`.
- **DAOs** por tabla: `manga_dao` (`upsert`, `get_by_key`, `list_library`,
  `set_library_flag`, `delete`), `chapter_dao` (`replace_for_manga`,
  `mark_read`, `list_for_manga`), `category_dao` (`list`, `add`, `rename`,
  `set_color`, `delete`, `assign`, `unassign`), `history_dao` (`upsert`,
  `recent`, `delete`), `download_dao` (`upsert`, `list_by_state`,
  `update_progress`, `set_state`).

### Settings (`core/settings.rs`)

```rust
#[derive(Serialize, Deserialize, Default)] pub struct Settings {
    pub theme: String,           // "dark" | "light" | "system"
    pub accent: String,          // hex
    pub default_source: Option<String>,
    pub download_concurrency: u32,
    pub library_view: String,    // "grid" | "list"
}
pub fn load() -> Settings;       // Lee XDG/configRoot/settings.json; default + crea si falta
pub fn save(&Settings);          // Escribe atómico (write to tmp + rename)
```

## 5. Arquitectura UI en Iced

### Estado central — `AppState`

```rust
struct AppState {
    daemon: DaemonClient,
    db: Arc<Mutex<Connection>>,
    downloads: DownloadManager,
    settings: Settings,
    theme: iced::Theme,
    screen: Screen,
    sources: Vec<Source>,
    library: Vec<Manga>,
    history: Vec<HistoryEntry>,
    error: Option<String>,
    version: PingReply,
}

enum Screen { Home, Browse(BrowseState), Details(DetailsState), Library,
              Reader(ReaderState), Downloads, Settings, Extensions }
```

### Mensaje global y `update`

```rust
#[derive(Debug, Clone)]
enum Message {
    DaemonStarted(Result<PingReply, DaemonError>),
    DaemonEvent(Result<(), DaemonError>),

    SourcesListed(Result<Vec<Source>, RpcException>),
    CatalogListed(Result<Vec<Manga>, RpcException>),
    MangaDetails(Result<Manga, RpcException>),
    ChapterPages(Result<Vec<Page>, RpcException>),
    PageUrl(Result<String, RpcException>),
    SourceHeaders(Result<HashMap<String,String>, RpcException>),

    LibraryLoaded(Result<Vec<Manga>, DbError>),
    MangaSaved(Result<(), DbError>),
    HistoryLoaded(Result<Vec<HistoryEntry>, DbError>),
    /* ... */

    DownloadProgress { manga_id: i64, chapter_url: String, done: i32, total: i32 },
    DownloadFinished(Result<DownloadOutcome, DownloadError>),
    DownloadEvent(DownloadEvent),   // del stream del manager

    Home(home::Message), Browse(browse::Message), Details(details::Message),
    Library(library::Message), Reader(reader::Message),
    Downloads(downloads::Message), Settings(settings::Message),
    NavigateTo(Screen), ErrorDismissed,
}

fn update(state: &mut AppState, msg: Message) -> Task<Message> { ... }
```

### Subscriptions continuas

```rust
fn subscription(&self) -> Subscription<Message> {
    Subscription::batch([
        daemon_socket_events().map(Message::DaemonEvent),
        download_manager_events().map(Message::DownloadEvent),
    ])
}
```

El **daemon socket reader** y el **download progress emitter** son streams
que viven mientras la app viva y emiten mensajes al `update`. Idiomático Iced.

### Commands puntuales

Cada interacción del usuario (refrescar lista, abrir detalles, pedir
páginas, marcar leído, agregar a biblioteca) lanza un `Task::perform`:

```rust
fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::Browse(browse::Message::Refresh(source)) =>
            Task::perform(state.daemon.clone().catalog_list(source, 0, None),
                          Message::CatalogListed),
        Message::CatalogListed(Ok(list)) => { state.browse.list = list; Task::none() }
        Message::CatalogListed(Err(e)) => { state.error = Some(e.to_string()); Task::none() }
        /* ... */
    }
}
```

### Pantallas (features/*)

| Pantalla | Equivalente Dart | Notas |
|---|---|---|
| `home` | `home/` | Start here reciente, biblioteca abreviada. |
| `browse` | `browse/` | Lista de fuentes + listado paginado con query. |
| `details` | `details/` | Cover, descripción, lista de capítulos, descargar. |
| `library` | `library/` | Grid/list de favoritos, filtrado por categoría. |
| `reader` | `reader/` | Visor: scroll continuo vertical o paginado; prefetch de páginas vecinas. |
| `downloads` | `downloads/` | Cola de descargas con estado/progreso. |
| `settings` | `settings/` | Tema, fuente default, concurrencia, vista biblioteca. |
| `extensions` | `extensions/` | Lista estática de fuentes (`sources.list`). |

### Imágenes (covers + páginas)

`core/net/` = wrapper sobre `reqwest` con caché en disco
(`XDG/cacheRoot/<sha256(url)>.<ext>`) usando `image` para decodificar y
`iced::widget::image::Handle` para render. Carga async vía `Task::perform`.

## 6. Download manager, distribución, testing, fases

### Download manager (`core/downloads/`)

```rust
pub struct DownloadManager {
    db: Arc<Mutex<Connection>>,
    daemon: DaemonClient,
    http: reqwest::Client,
    root: PathBuf,
    concurrency: u32,
    progress_tx: tokio::sync::mpsc::UnboundedSender<DownloadEvent>,
}
pub enum DownloadEvent { Queued(i64, String),
                         Progress{i64, String, i32, i32},
                         Done(i64, String),
                         Errored(i64, String, String) }
```

**Algoritmo:**

1. `enqueue(manga, chapter)`: inserta/actualiza fila `download` con
   state=`Queued`, notifica.
2. Bucle del manager (task perpetua tokio): toma hasta `concurrency`
   capítulos en `Queued`, los pasa a `Downloading`; para cada uno:
   - `daemon.chapter_pages(source, chapter)` → lista de `Page`.
   - Para cada página: obtiene URL final (`daemon.page_url`), descarga
     imagen con headers de la fuente (`daemon.source_headers` + `Referer`),
     guarda en `downloadsRoot/<source>/<mangaHash>/<chapterHash>/<NN>.<ext>`.
   - Por cada página: `download_dao.update_progress` + emite `Progress`.
   - Al terminar: state=`Done`. En error: state=`Error` + mensaje.
3. El stream de `DownloadEvent` se expone como `Subscription` Iced.

`mangaHash`/`chapterHash` = `sha256(manga.key())` / `sha256(chapter.url)`
truncado (mismo esquema del Dart).

### Distribución — AppImage

Sin Flutter, `make_universal.sh` simplifica:

```
AppDir/
  usr/bin/bakeneko                # binario Rust (cargo build --release)
  usr/bin/bakeneko-daemon.jar     # JAR del daemon
  usr/jre/...                     # JRE (temurin 21)
  AppRun                          # wrapper (AppImageKit)
```

`cargo build --release` produce binario nativo. `shell.nix` se reduce a:
`rustup`, `cargo`, `jdk21`, `pkg-config` y deps gráficas del backend de
Iced. Build: `cargo build --release` + `gradlew shadowJar` + `make_universal.sh`.

### Testing — espejo de `app/test/`

| Test Dart | Test Rust |
|---|---|
| `xdg_test.dart` | `xdg_test.rs` |
| `settings_test.dart` | `settings_test.rs` |
| `rpc_test.dart` | `rpc_test.rs` |
| `database_test.dart` | `database_test.rs` (migraciones idempotentes) |
| `dao_test.dart` (varios) | DAOs con conexión `:memory:` |
| `download_manager_test.dart` | cola con daemon mock (trait `MangaSourceApi`) |

Para hacer testeable el `DaemonClient` sin JVM, se abstrae detrás de un
**trait** `MangaSourceApi` con los 7 métodos; en producción la impl concreta
habla al socket, en tests una impl mock devuelve datos de fixtures.

### Plan de fases

| Fase | Entregable | Verificación |
|---|---|---|
| **0** | Esqueleto crate, `xdg`, `settings`, `rpc` (encode/decode) con tests. | `cargo test` verde. |
| **1** | `DaemonClient` real + `models` + `ping`/`sources.list`/`catalog.list`; UI mínima en Iced: lista de fuentes + catálogo. | App abre, muestra fuentes, lista MangaDex. |
| **2** | `db` (schema + migraciones + 5 DAOs) + pantallas `library`, `details`, `history`. | Persiste favoritos, historial. |
| **3** | `reader` (páginas) + `core/net` caché imágenes. | Lee capítulo descargando páginas. |
| **4** | `downloads` (DownloadManager + Subscription) + pantalla downloads. | Descarga capítulos con progreso. |
| **5** | Settings UI + extensions + pulido thema + cierre limpio. | Build AppImage final. |

Esto traduce **todo** el Dart, en orden de dependencias crecientes,
dejando descargas (lo más complejo) para el final.

## 7. Decisiones de diseño resumidas

| Decisión | Valor | Por qué |
|---|---|---|
| Daemon | Sin cambios (Kotlin JVM + `futon-parsers`) | Reaprovecha la librería de parsers sin reescribirla. |
| UI | Iced (pure Rust, Elm-like) | Binario autónomo, idiomático, mapea a Riverpod. |
| Runtime async | tokio | Default de Iced; necesario para `tokio::process` y `tokio::net::UnixStream`. |
| SQLite | rusqlite (sync) | Traducción fiel del drift; `spawn_blocking` en contexto async. |
| Arquitectura | Subscriptions + commands | Idiomático Iced; separa streams continuos de acciones puntuales. |
| Protocolo RPC | Byte-compatible | Daemon sin cambios, interoperable con versión Dart. |
| Schema DB | Idéntica | Compatibility con DBs Dart existentes. |
| Distribución | AppImage (Rust + JAR + JRE) | Mismo formato final, mucho más liviano. |

## 8. Fuera de alcance (YAGNI)

- No se reescribe ningún parser de fuentes en Rust: ese trabajo ya lo hace
  `futon-parsers` y reescribirlo es trabajo enorme sin retorno.
- No se añade soporte multiplataforma (macOS/Windows) en esta fase: el
  proyecto es explícitamente Linux-nativo.
- No se añaden nuevas features (lectura offline avanzada, sync entre
  dispositivos, OPDS, etc.) — solo se traduce lo existente.
- No se migra el build a un workspace multi-crate: single-crate es
  suficiente y más simple de aprender.