# Design — Code Review Native Plugin

## Status

- **Date**: 2026-08-09
- **Decision**: Go (staged) — pending graphify-core v1 contract alignment
- **Evaluation basis**: oracle architecture review of upstream `code-review-graph` (Python) vs native Rust embedded plugin

## Architecture

### 定位

`graphify-plugin-review` 是 Graphify 內嵌型 Rust crate，實作 `GraphifyPlugin` trait，
分析直接運行於 Graphify Core 記憶體內的 petgraph（`GraphOutput`）之上 — 不建獨立圖、
不持 SQLite、不跑自己的解析管線（共用 Graphify 的圖）。

```
graphify-mcp (GraphifyRust)
  └─ 註冊 review* tools
       └─ graphify-plugin-review (embedded crate, implements GraphifyPlugin)
            └─ 讀 Graphify Core 記憶體 petgraph (GraphOutput)
                 └─ git diff → 變更節點 → BFS impact → review context
```

### 與 Graphify Core 契約對齊

- `WorkspaceContext{workspace_key, workspace_name, root_path, timestamp}` — `bind` 時注入
- `workspace_key` 為跨 plugin 對齊鍵（handoff / review / opendoc 共用）
- 同步 `GraphifyPlugin` 介面：`get_id / bind / get_workspace_key / sync_toon / on_graph_updated`

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
| `review_entrypoints` | `detect_entry_points` | 無 incoming CALLS 的函數 + 框架裝飾器 |
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
