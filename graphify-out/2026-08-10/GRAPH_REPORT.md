# Graph Report - bakeneko-rs  (2026-08-10)

## Corpus Check
- 33 files · ~19,093 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 287 nodes · 558 edges · 26 communities (24 shown, 2 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 7 edges (avg confidence: 0.79)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `65b427fc`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- DaemonClient
- DaemonClient (Rust, spawn java + Unix socket)
- File Structure
- DbError
- main
- app.rs
- models.rs
- category_dao.rs
- RpcResponse
- Database
- Manga
- Chapter
- settings.rs
- Xdg
- dao_test.rs

## God Nodes (most connected - your core abstractions)
1. `DbError` - 39 edges
2. `DaemonClient` - 27 edges
3. `DaemonError` - 22 edges
4. `Manga` - 21 edges
5. `Xdg` - 14 edges
6. `Chapter` - 13 edges
7. `File Structure` - 13 edges
8. `MockDaemon` - 9 edges
9. `Database` - 8 edges
10. `db_blocking()` - 8 edges

## Surprising Connections (you probably didn't know these)
- `corrupt_file_yields_default()` --calls--> `load()`  [INFERRED]
  tests/settings_test.rs → src/core/settings.rs
- `missing_file_yields_default()` --calls--> `load()`  [INFERRED]
  tests/settings_test.rs → src/core/settings.rs
- `MockDaemon` --implements--> `MangaSourceApi`  [EXTRACTED]
  tests/daemon_client_test.rs → src/core/daemon/api.rs
- `sample_manga()` --references--> `Manga`  [EXTRACTED]
  tests/dao_test.rs → src/core/models.rs
- `roundtrip_preserves_values()` --calls--> `load()`  [INFERRED]
  tests/settings_test.rs → src/core/settings.rs

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **JSON-RPC + opaque blob round-trip flow (Rust client -> UDS -> Kotlin daemon)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daemonclient, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_rpcrequest, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_opaque_blob_roundtrip, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_byte_compat_rpc, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_jvm_daemon_unchanged [EXTRACTED 1.00]
- **Iced Elm-like architecture (AppState + Message/update + Subscriptions/Commands)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_appstate, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_message_enum, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_subscriptions_commands, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_iced_runtime [EXTRACTED 1.00]
- **Download pipeline (manager -> daemon pages -> http -> dao progress -> event stream)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_downloadmanager, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daemonclient, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_net_image_cache, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daos, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_downloadevent [EXTRACTED 1.00]

## Communities (26 total, 2 thin omitted)

### Community 0 - "DaemonClient"
Cohesion: 0.13
Nodes (20): AtomicU64, Child, Send, Sender, MangaSourceApi, DaemonClient, Arc, HashMap (+12 more)

### Community 2 - "DaemonClient (Rust, spawn java + Unix socket)"
Cohesion: 0.10
Nodes (24): AppImage distribution (Rust + JAR + JRE), AppState (central Iced model), Bakeneko Rust Port, Byte-compatible JSON-RPC 2.0 protocol, DaemonClient (Rust, spawn java + Unix socket), DAOs (manga_dao/chapter_dao/category_dao/history_dao/download_dao), DownloadEvent stream, DownloadManager (queue + progress + tokio task) (+16 more)

### Community 3 - "File Structure"
Cohesion: 0.11
Nodes (18): appimagetool (descargar si falta), AppRun wrapper, Bakeneko → Rust Implementation Plan, Bakeneko-Universal: binario Rust + JAR daemon + JRE. Sin Flutter., File Structure, Global Constraints, Task 10: core/net — descarga de imágenes con caché en disco, Task 11: app.rs — AppState, Message, update, subscription, view (+10 more)

### Community 4 - "DbError"
Cohesion: 0.18
Nodes (23): ds_to_str(), list(), list_by_state(), row_to_entry(), Connection, Result, Row, Vec (+15 more)

### Community 8 - "models.rs"
Cohesion: 0.13
Nodes (16): HistoryEntry, MangaRef, Page, PingReply, String, Source, fake_server(), mock_satisfies_api() (+8 more)

### Community 9 - "category_dao.rs"
Cohesion: 0.28
Nodes (15): add(), assign(), delete(), for_manga(), list(), rename(), row_to_category(), Connection (+7 more)

### Community 11 - "RpcResponse"
Cohesion: 0.14
Nodes (9): RpcErr, RpcRequest, RpcResponse, Error, Option, Result, Self, String (+1 more)

### Community 13 - "Database"
Cohesion: 0.22
Nodes (11): F, Database, db_blocking(), Arc, Connection, Mutex, Option, Path (+3 more)

### Community 16 - "Manga"
Cohesion: 0.30
Nodes (15): delete(), get_by_id(), get_by_key(), get_id_by_key(), list_library(), row_to_manga(), Connection, Option (+7 more)

### Community 18 - "Chapter"
Cohesion: 0.24
Nodes (13): Map, chapter_blob_json(), list_for_manga(), mark_read(), replace_for_manga(), row_to_chapter(), Connection, Result (+5 more)

### Community 20 - "settings.rs"
Cohesion: 0.20
Nodes (13): Default, load(), Option, PathBuf, Result, Self, String, save() (+5 more)

### Community 21 - "Xdg"
Cohesion: 0.18
Nodes (4): PathBuf, Result, String, Xdg

### Community 24 - "dao_test.rs"
Cohesion: 0.49
Nodes (9): categories_and_assignment(), chapters_replace_and_mark_read(), download_states_transition(), history_upsert_recent(), library_flag_roundtrip(), manga_upsert_and_get_by_key(), Connection, sample_manga() (+1 more)

## Knowledge Gaps
- **21 isolated node(s):** `App`, `Global Constraints`, `Task 1: Cargo scaffold + entry point`, `Task 2: core/xdg.rs — XDG paths`, `Task 3: core/settings.rs — settings.json load/save` (+16 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **2 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `DbError` connect `DbError` to `DaemonClient`, `category_dao.rs`, `Database`, `Manga`, `Chapter`?**
  _High betweenness centrality (0.178) - this node is a cross-community bridge._
- **Why does `DaemonError` connect `DaemonClient` to `models.rs`, `DbError`?**
  _High betweenness centrality (0.096) - this node is a cross-community bridge._
- **Why does `Manga` connect `Manga` to `DaemonClient`, `models.rs`, `category_dao.rs`, `Chapter`, `dao_test.rs`?**
  _High betweenness centrality (0.085) - this node is a cross-community bridge._
- **What connects `App`, `Global Constraints`, `Task 1: Cargo scaffold + entry point` to the rest of the system?**
  _21 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `DaemonClient` be split into smaller, more focused modules?**
  _Cohesion score 0.12612612612612611 - nodes in this community are weakly interconnected._
- **Should `DaemonClient (Rust, spawn java + Unix socket)` be split into smaller, more focused modules?**
  _Cohesion score 0.10144927536231885 - nodes in this community are weakly interconnected._
- **Should `File Structure` be split into smaller, more focused modules?**
  _Cohesion score 0.10526315789473684 - nodes in this community are weakly interconnected._