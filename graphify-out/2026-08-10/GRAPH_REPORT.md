# Graph Report - bakeneko-rs  (2026-08-10)

## Corpus Check
- 47 files · ~27,711 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 484 nodes · 973 edges · 31 communities (30 shown, 1 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 9 edges (avg confidence: 0.79)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `53fbdfdc`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- DaemonError
- browse/mod.rs
- DaemonClient (Rust, spawn java + Unix socket)
- File Structure
- models.rs
- Screen
- library/mod.rs
- home/mod.rs
- DbError
- RpcResponse
- FakeDaemon
- MockDaemon
- Manga
- MangaSourceApi
- Settings
- Xdg
- Database
- app.rs
- dao_test.rs
- AppState
- update
- details/mod.rs
- view
- make_universal.sh

## God Nodes (most connected - your core abstractions)
1. `DbError` - 46 edges
2. `AppState` - 39 edges
3. `DaemonError` - 36 edges
4. `Manga` - 35 edges
5. `DaemonClient` - 33 edges
6. `Chapter` - 22 edges
7. `File Structure` - 20 edges
8. `Message` - 18 edges
9. `Xdg` - 14 edges
10. `MangaSourceApi` - 12 edges

## Surprising Connections (you probably didn't know these)
- `corrupt_file_yields_default()` --calls--> `load()`  [INFERRED]
  tests/settings_test.rs → src/core/settings.rs
- `missing_file_yields_default()` --calls--> `load()`  [INFERRED]
  tests/settings_test.rs → src/core/settings.rs
- `MockDaemon` --implements--> `MangaSourceApi`  [EXTRACTED]
  tests/daemon_client_test.rs → src/core/daemon/api.rs
- `FakeDaemon` --implements--> `MangaSourceApi`  [EXTRACTED]
  tests/download_manager_test.rs → src/core/daemon/api.rs
- `sample_manga()` --references--> `Manga`  [EXTRACTED]
  tests/dao_test.rs → src/core/models.rs

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **JSON-RPC + opaque blob round-trip flow (Rust client -> UDS -> Kotlin daemon)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daemonclient, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_rpcrequest, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_opaque_blob_roundtrip, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_byte_compat_rpc, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_jvm_daemon_unchanged [EXTRACTED 1.00]
- **Iced Elm-like architecture (AppState + Message/update + Subscriptions/Commands)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_appstate, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_message_enum, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_subscriptions_commands, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_iced_runtime [EXTRACTED 1.00]
- **Download pipeline (manager -> daemon pages -> http -> dao progress -> event stream)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_downloadmanager, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daemonclient, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_net_image_cache, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daos, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_downloadevent [EXTRACTED 1.00]

## Communities (31 total, 1 thin omitted)

### Community 0 - "DaemonError"
Cohesion: 0.12
Nodes (19): AsyncMutex, AtomicU64, Child, DaemonClient, Arc, HashMap, Mutex, Option (+11 more)

### Community 1 - "browse/mod.rs"
Cohesion: 0.24
Nodes (10): Message, AppMessage, Element, Option, String, Task, Vec, State (+2 more)

### Community 2 - "DaemonClient (Rust, spawn java + Unix socket)"
Cohesion: 0.10
Nodes (24): AppImage distribution (Rust + JAR + JRE), AppState (central Iced model), Bakeneko Rust Port, Byte-compatible JSON-RPC 2.0 protocol, DaemonClient (Rust, spawn java + Unix socket), DAOs (manga_dao/chapter_dao/category_dao/history_dao/download_dao), DownloadEvent stream, DownloadManager (queue + progress + tokio task) (+16 more)

### Community 3 - "File Structure"
Cohesion: 0.09
Nodes (22): Bakeneko → Rust Implementation Plan, File Structure, Global Constraints, Task 10: core/net — descarga de imágenes con caché en disco, Task 11: app.rs — AppState, Message, update, subscription, view, Task 12: features/browse + features/home, Task 13: Daemon lifecycle — start en main + socket-watch subscription, Task 14: features/library + features/details (con DB) (+14 more)

### Community 4 - "models.rs"
Cohesion: 0.08
Nodes (27): ds_to_str(), list(), list_by_state(), row_to_entry(), Connection, Result, Row, Vec (+19 more)

### Community 6 - "Screen"
Cohesion: 0.27
Nodes (6): M, Self, Screen, NavMsg, Element, view()

### Community 7 - "library/mod.rs"
Cohesion: 0.29
Nodes (9): details_load(), Message, AppMessage, Element, Option, Task, State, update() (+1 more)

### Community 8 - "home/mod.rs"
Cohesion: 0.27
Nodes (9): Message, AppMessage, Element, Result, Task, Vec, State, update() (+1 more)

### Community 9 - "DbError"
Cohesion: 0.13
Nodes (35): Clone, add(), assign(), delete(), for_manga(), list(), rename(), row_to_category() (+27 more)

### Community 11 - "RpcResponse"
Cohesion: 0.14
Nodes (9): RpcErr, RpcRequest, RpcResponse, Error, Option, Result, Self, String (+1 more)

### Community 13 - "FakeDaemon"
Cohesion: 0.21
Nodes (9): enqueue_then_poll_marks_done(), FakeDaemon, HashMap, Option, Result, String, Vec, sample_chapter() (+1 more)

### Community 16 - "MockDaemon"
Cohesion: 0.20
Nodes (10): fake_server(), mock_satisfies_api(), MockDaemon, real_client_pings_fake_server(), HashMap, Option, Path, Result (+2 more)

### Community 18 - "Manga"
Cohesion: 0.07
Nodes (44): Map, chapter_blob_json(), list_for_manga(), mark_read(), replace_for_manga(), row_to_chapter(), Connection, Result (+36 more)

### Community 19 - "MangaSourceApi"
Cohesion: 0.09
Nodes (28): Client, Handle, Receiver, Runtime, Send, MangaSourceApi, DownloadManager, Inner (+20 more)

### Community 20 - "Settings"
Cohesion: 0.11
Nodes (24): Color, Message, load(), Default, Option, PathBuf, Result, Self (+16 more)

### Community 21 - "Xdg"
Cohesion: 0.18
Nodes (4): PathBuf, Result, String, Xdg

### Community 22 - "Database"
Cohesion: 0.22
Nodes (11): F, Database, db_blocking(), Arc, Connection, Mutex, Option, Path (+3 more)

### Community 23 - "app.rs"
Cohesion: 0.17
Nodes (14): From, Loaded, Message, Element, Result, Task, Vec, subscription() (+6 more)

### Community 24 - "dao_test.rs"
Cohesion: 0.49
Nodes (9): categories_and_assignment(), chapters_replace_and_mark_read(), download_states_transition(), history_upsert_recent(), library_flag_roundtrip(), manga_upsert_and_get_by_key(), Connection, sample_manga() (+1 more)

### Community 26 - "AppState"
Cohesion: 0.25
Nodes (8): AppState, Arc, Connection, Default, Mutex, Option, String, State

### Community 27 - "update"
Cohesion: 0.32
Nodes (7): Message, AppMessage, Element, String, Task, update(), view()

### Community 28 - "details/mod.rs"
Cohesion: 0.40
Nodes (5): AppMessage, Element, Task, update(), view()

### Community 29 - "view"
Cohesion: 0.67
Nodes (3): AppMessage, Element, view()

## Knowledge Gaps
- **25 isolated node(s):** `make_universal.sh script`, `Global Constraints`, `Task 1: Cargo scaffold + entry point`, `Task 2: core/xdg.rs — XDG paths`, `Task 3: core/settings.rs — settings.json load/save` (+20 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppState` connect `AppState` to `DaemonError`, `browse/mod.rs`, `models.rs`, `Screen`, `library/mod.rs`, `home/mod.rs`, `Manga`, `MangaSourceApi`, `Settings`, `app.rs`, `update`, `details/mod.rs`, `view`?**
  _High betweenness centrality (0.184) - this node is a cross-community bridge._
- **Why does `DbError` connect `DbError` to `DaemonError`, `models.rs`, `home/mod.rs`, `Manga`, `MangaSourceApi`, `Database`, `app.rs`?**
  _High betweenness centrality (0.150) - this node is a cross-community bridge._
- **Why does `Manga` connect `Manga` to `DaemonError`, `browse/mod.rs`, `models.rs`, `library/mod.rs`, `home/mod.rs`, `DbError`, `FakeDaemon`, `MockDaemon`, `MangaSourceApi`, `app.rs`, `dao_test.rs`, `AppState`?**
  _High betweenness centrality (0.125) - this node is a cross-community bridge._
- **What connects `make_universal.sh script`, `Global Constraints`, `Task 1: Cargo scaffold + entry point` to the rest of the system?**
  _25 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `DaemonError` be split into smaller, more focused modules?**
  _Cohesion score 0.11875843454790823 - nodes in this community are weakly interconnected._
- **Should `DaemonClient (Rust, spawn java + Unix socket)` be split into smaller, more focused modules?**
  _Cohesion score 0.10144927536231885 - nodes in this community are weakly interconnected._
- **Should `File Structure` be split into smaller, more focused modules?**
  _Cohesion score 0.08695652173913043 - nodes in this community are weakly interconnected._