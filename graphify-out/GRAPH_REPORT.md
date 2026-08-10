# Graph Report - .  (2026-08-10)

## Corpus Check
- Corpus is ~2,542 words - fits in a single context window. You may not need a graph.

## Summary
- 24 nodes · 28 edges · 5 communities
- Extraction: 89% EXTRACTED · 11% INFERRED · 0% AMBIGUOUS · INFERRED: 3 edges (avg confidence: 0.78)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Project Scope & Plan
- Daemon IPC & DTOs
- Downloads & App State
- Iced UI Architecture
- SQLite Persistence

## God Nodes (most connected - your core abstractions)
1. `DaemonClient (Rust, spawn java + Unix socket)` - 8 edges
2. `DAOs (manga_dao/chapter_dao/category_dao/history_dao/download_dao)` - 4 edges
3. `DownloadManager (queue + progress + tokio task)` - 4 edges
4. `Bakeneko Rust Port` - 3 edges
5. `Original Bakeneko (Flutter/Dart + Kotlin JVM)` - 3 edges
6. `Manga/Chapter/Page/Source/Category/HistoryEntry/MangaRef models` - 3 edges
7. `SCHEMA_SQL (identical to schema.dart)` - 3 edges
8. `AppState (central Iced model)` - 3 edges
9. `Message enum + update (Elm-like)` - 3 edges
10. `Byte-compatible JSON-RPC 2.0 protocol` - 2 edges

## Surprising Connections (you probably didn't know these)
- `Manga/Chapter/Page/Source/Category/HistoryEntry/MangaRef models` --shares_data_with--> `DAOs (manga_dao/chapter_dao/category_dao/history_dao/download_dao)`  [INFERRED]
  docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md → docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md  _Bridges community 1 → community 4_
- `SCHEMA_SQL (identical to schema.dart)` --conceptually_related_to--> `Original Bakeneko (Flutter/Dart + Kotlin JVM)`  [EXTRACTED]
  docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md → docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md  _Bridges community 0 → community 4_
- `DaemonClient (Rust, spawn java + Unix socket)` --implements--> `Byte-compatible JSON-RPC 2.0 protocol`  [EXTRACTED]
  docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md → docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md  _Bridges community 0 → community 1_
- `AppState (central Iced model)` --shares_data_with--> `DaemonClient (Rust, spawn java + Unix socket)`  [EXTRACTED]
  docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md → docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md  _Bridges community 1 → community 2_
- `DownloadManager (queue + progress + tokio task)` --shares_data_with--> `DAOs (manga_dao/chapter_dao/category_dao/history_dao/download_dao)`  [EXTRACTED]
  docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md → docs/superpowers/specs/2026-08-10-bakeneko-rust-port-design.md  _Bridges community 4 → community 2_

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **JSON-RPC + opaque blob round-trip flow (Rust client -> UDS -> Kotlin daemon)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daemonclient, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_rpcrequest, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_opaque_blob_roundtrip, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_byte_compat_rpc, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_jvm_daemon_unchanged [EXTRACTED 1.00]
- **Iced Elm-like architecture (AppState + Message/update + Subscriptions/Commands)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_appstate, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_message_enum, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_subscriptions_commands, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_iced_runtime [EXTRACTED 1.00]
- **Download pipeline (manager -> daemon pages -> http -> dao progress -> event stream)** — docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_downloadmanager, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daemonclient, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_net_image_cache, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_daos, docs_superpowers_specs_2026_08_10_bakeneko_rust_port_design_downloadevent [EXTRACTED 1.00]

## Communities (5 total, 0 thin omitted)

### Community 0 - "Project Scope & Plan"
Cohesion: 0.33
Nodes (6): AppImage distribution (Rust + JAR + JRE), Bakeneko Rust Port, Byte-compatible JSON-RPC 2.0 protocol, Kotlin JVM daemon (unchanged, futon-parsers), Original Bakeneko (Flutter/Dart + Kotlin JVM), Phased port plan (0-5, dependency order)

### Community 1 - "Daemon IPC & DTOs"
Cohesion: 0.40
Nodes (6): DaemonClient (Rust, spawn java + Unix socket), MangaSourceApi trait (test abstraction for DaemonClient), Manga/Chapter/Page/Source/Category/HistoryEntry/MangaRef models, Opaque blob round-trip (DAO/DTO preserve raw JSON), RpcRequest / RpcResponse / RpcException / RpcCodes, tokio async runtime

### Community 2 - "Downloads & App State"
Cohesion: 0.40
Nodes (5): AppState (central Iced model), DownloadEvent stream, DownloadManager (queue + progress + tokio task), Settings (settings.json serde_json-compatible), core/xdg.rs (XDG paths + /proc uid)

### Community 3 - "Iced UI Architecture"
Cohesion: 0.50
Nodes (4): Iced (pure-Rust Elm-like UI), Message enum + update (Elm-like), core/net: reqwest + disk image cache (sha256 paths), Subscriptions + Commands architecture (Iced)

### Community 4 - "SQLite Persistence"
Cohesion: 1.00
Nodes (3): DAOs (manga_dao/chapter_dao/category_dao/history_dao/download_dao), rusqlite (sync, spawn_blocking in async), SCHEMA_SQL (identical to schema.dart)

## Knowledge Gaps
- **4 isolated node(s):** `RpcRequest / RpcResponse / RpcException / RpcCodes`, `MangaSourceApi trait (test abstraction for DaemonClient)`, `core/net: reqwest + disk image cache (sha256 paths)`, `core/xdg.rs (XDG paths + /proc uid)`
  These have ≤1 connection - possible missing edges or undocumented components.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `DaemonClient (Rust, spawn java + Unix socket)` connect `Daemon IPC & DTOs` to `Project Scope & Plan`, `Downloads & App State`?**
  _High betweenness centrality (0.503) - this node is a cross-community bridge._
- **Why does `DownloadManager (queue + progress + tokio task)` connect `Downloads & App State` to `Daemon IPC & DTOs`, `SQLite Persistence`?**
  _High betweenness centrality (0.414) - this node is a cross-community bridge._
- **Why does `Original Bakeneko (Flutter/Dart + Kotlin JVM)` connect `Project Scope & Plan` to `SQLite Persistence`?**
  _High betweenness centrality (0.315) - this node is a cross-community bridge._
- **What connects `RpcRequest / RpcResponse / RpcException / RpcCodes`, `MangaSourceApi trait (test abstraction for DaemonClient)`, `core/net: reqwest + disk image cache (sha256 paths)` to the rest of the system?**
  _4 weakly-connected nodes found - possible documentation gaps or missing edges._