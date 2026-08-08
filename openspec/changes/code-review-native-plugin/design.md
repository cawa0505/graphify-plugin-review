# Design — Code Review Native Plugin

## Status

- **Date**: 2026-08-09
- **Decision**: Go (staged) — pending graphify-core v1 contract alignment
- **Evaluation basis**: oracle architecture review of upstream `code-review-graph` (Python) vs native Rust embedded plugin

## 定位：雙軌平行 + 漸進式演進（Dual-Track Evolutionary）

`graphify-plugin-review` 是一個 monorepo，以同一個 Python 起點（`code-review-graph`）
展示兩條平行的進化路徑，以及 Graphify SDK 的第一個 first-class client：

| 軌 | 組件 | 落點 | 定位 |
|----|------|------|------|
| **A** | Rust 原生 Review Plugin | `crates/graphify-plugin-review/` | 16ms 極致效能、單一二進位、記憶體 petgraph 直接操作 |
| **B** | Graphify Python SDK (`graphify-sdk`) | `sdk/` | 官方對外 Python 打底基礎設施（高階 async API） |
| **B** | 改版 Python Code Review MCP | `python/review-mcp/` | SDK 的第一個 first-class client（拓撲感知審查） |

```
graphify-plugin-review/
├── crates/
│   └── graphify-plugin-review/   # [軌 A] embedded crate, implements GraphifyPlugin
├── sdk/                          # [軌 B] graphify-sdk（Python 官方 SDK，可抽出結構）
│   ├── pyproject.toml            #   package: graphify-sdk
│   └── graphify_sdk/             #   client.py / api.py / workspace.py
├── python/
│   └── review-mcp/               # [軌 B] 改版 Python Review MCP（SDK first-class client）
├── legacy/code-review-graph/     # 原始 Python tool（fork, reference, 不動）
├── docs/integration/             # SDK 整合進不同語言 MCP 的過程紀錄
└── openspec/                     # 本變更的 proposal / design / tasks
```

### [軌 A] 原生 Rust rewrite 定位

`crates/graphify-plugin-review` 分析直接運行於 Graphify Core 記憶體內的 petgraph
（`GraphOutput`）之上 — 不建獨立圖、不持 SQLite、不跑自己的解析管線（共用 Graphify 的圖）。

```
graphify-mcp (GraphifyRust)
  └─ 註冊 review* tools
       └─ graphify-plugin-review (embedded crate, implements GraphifyPlugin)
            └─ 讀 Graphify Core 記憶體 petgraph (GraphOutput)
                 └─ on_graph_updated(modified_nodes) → BFS impact → review context
```

### [軌 B] graphify-sdk 定位

- `GraphifyClient(workspace_key)`：自動處理與 graphify-mcp 的 Stdio/JSON-RPC 通訊與進程生命週期
- `get_blast_radius(git_diff/files, depth=3)`：索取經 `.toon` 壓縮的衝擊半徑拓撲
- `query_symbol_topology(symbol_name)`：查詢符號上下游呼叫鏈拓撲
- 自動透傳 `workspace_key`，Python 開發者/Agent 無需手動處理底層 Schema
- **可抽出結構**：pyproject 與模組獨立，API 穩定後 `git mv` 即可無痛抽出為獨立 repo

### [軌 B] 改版 Python Review MCP 定位

- 100% 沿用成熟 Python 資產（Skills / Prompt 範本 / 安全與效能檢查）
- 收到 PR / git diff 時透過 graphify-sdk 發 `get_blast_radius(git_diff)` 取得拓撲地圖
- 將 `{{ topology_impact_toon }}` 注入 Review Prompt — 讓 LLM 能對照 `.toon` 拓撲
  精準警告「這行修改會破壞 2 階以外遠端模組的呼叫鏈（Breaking Change Risk）」
- 暴露 `review_pull_request(workspace_key, git_diff, modified_files)` 供
  OpenCode / Cursor / LLM Agent 呼叫

### 與 Graphify Core 契約對齊（v1 已驗證）

直接驗證自 `graphify-core/src/`（2026-08-09）：

- `GraphifyPlugin` trait：`get_id / bind / get_workspace_key / sync_toon(Option<Vec<u8>>) -> Vec<u8> / on_graph_updated`
- `WorkspaceContext{workspace_key, workspace_name, root_path, timestamp}` — `bind` 時注入
- `GraphUpdateEvent{workspace_key, modified_nodes: Vec<NodeId>, event}` — **變更偵測來源是
  core 主動推送的 `on_graph_updated`，plugin 不需要自己跑 git diff**（Phase 2 若要
  「任意 commit 間比較」才需另議 git 歷史）
- `NodeId(pub String)`；`Node{id, label, file_type, kind, language, source_file, start_line, end_line, ...}`
- `Edge{source, target, relation: String, source_file, confidence, ...}` — relation 為**小寫**
  （`"calls"` / `"contains"` / `"exports"` / `"imports"` / `"member_of"`）
- `GraphOutput{nodes, edges, metadata}`；`build_graph` 建 DiGraph + node_map
- `query_bfs` / `find_shortest_path` — BFS 與最短路徑 primitive 已公開
- `from_toon` / `to_toon` — `.toon` 序列化與 graphify-core 共用
- `workspace_key` 為跨 plugin 對齊鍵（handoff / review / opendoc 共用）

### Forward Contract：`analyze_diff_impact`（[待討論]）

[軌 A] 提出新增 trait 方法（v1 目前無此方法；屬 GraphifyRust `graphify-core/src/plugin.rs`
領域，本文件僅記錄提案，未對齊前不當作已定案）：

```rust
fn analyze_diff_impact(&self, ctx: &WorkspaceContext, git_diff: &str, graph: &GraphOutput) -> ReviewOutput;
```

- 記憶體級 git diff 拓撲解析：直接接收 `git_diff` 與 `&GraphOutput` 指針
- 毫秒級解析 diff 中被修改的 Struct/Function 符號，在 petgraph 執行 BFS（1–3 階半徑）
- 找出直接與間接受衝擊的下游呼叫鏈（Callers）
- 原生 Prompt 上下文合成：在 Rust 記憶體內將 git diff 與 `.toon` 衝擊子圖合併，
  直接產出含拓撲資訊的 Review 上下文（MVP 用 `format!`，暫不引入 Tera/Jinja2）
- 對齊時機：GraphifyRust 將 plugin.rs 補上此方法後，Task 3 的 trait 實作才能完整落地

### SDK 語言順序（supersedes D5）

SDK roadmap D5 原順序 TS → Python → PHP → Rust → Go（暫緩）。本變更將 **Python 提前**
（graphify-sdk 作為官方 Python 對外基礎設施），supersede 該順序的 Python 項。

### 零 Mock 原則

分析一律對真實記憶體圖與真實 git 狀態執行；無 fixture、無偽造數據、無快取替身。

### 效能預算

- 16ms BFS impact trace 為**常見路徑的效能目標**（非硬性 SLA）
- MVP 驗證目標（對比 Python upstream）：

| 操作 | Python 現況 (ms) | Rust MVP 目標 (ms) | 改善 |
|------|-----------------|-------------------|------|
| 單檔案 AST 解析 | 100–300 | 80–250 | 1.2–1.5x（共用 tree-sitter C lib，GIL 消除） |
| impact BFS（1000 節點） | 50–200 | 2–10 | 10–20x |
| flow trace | 20–100 | 2–5 | 10–20x |
| symbol search（10k 節點） | 100–500 | 5–20 | 5–20x |

依據：Python 版開銷在 SQLite 點查詢 + NetworkX 圖遍歷 + TOON 序列化，這些在 Rust
記憶體圖上直接消除；AST 解析共用 tree-sitter C 庫，提升有限。冷啟動（2–5s Python
依賴載入）與記憶體（50–70% 重複圖儲存）在嵌入式架構下歸零。

## 工具收斂

### 核心 review* 工具（Phase 1 MVP，4 個）

| 工具 | 對應上游 | 資料來源 |
|------|---------|---------|
| `review_impact` | `get_impact_radius` | petgraph BFS from changed nodes, depth 2 |
| `review_entrypoints` | `detect_entry_points` | 無 incoming `calls` 的函數 + 框架裝飾器 |
| `review_flows` | `trace_flows` | 執行路徑追蹤 + 關鍵度 |
| `review_callers` | `get_edges_by_target` | 反向呼叫追蹤 |

### Phase 2（視需求）

`review_tests`（transitive test 覆蓋）、`review_symbols`（符號搜尋）

### 明確不實作（YAGNI）

- Python 30 tools 的 embeddings / community detection / D3 視覺化 / 多語言語義分析
- 自建 SQLite graph store

## [待討論] 未決項目

- Embeddings 語義搜尋：Graphify 圖為純結構；如需語義檢索，由 Graphify llm 層提供或
  plugin 可選掛載（ONNX / 外部服務）— 待 GraphifyRust RAG boundary 決策落地。
- Community detection：petgraph 演算法可行，但僅在真實需求出現時實作。
- 上游 `code-review-graph` 是否保留向後相容橋接（同 handoff 決議：無 MCP 註冊需求即無橋接必要）。
