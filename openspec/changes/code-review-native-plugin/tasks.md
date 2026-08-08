# Tasks — Code Review Native Plugin

> 文件先行：Task 1 為文件與契約對齊，Task 2 起才動程式碼。
> 阻塞相依：graphify-core v1 `GraphifyPlugin` trait / `GraphOutput` 需存在（GraphifyRust 側，已確認）。
> 本 repo 為雙重角色 monorepo：SDK 整合示範（sdk/ + docs/integration/）+ 原生 Rust rewrite（crates/）。

## Task 1: 文件與契約對齊（文件先行）
- [x] 上游研究：`legacy/code-review-graph/`（tirth8205/code-review-graph v2.3.6 fork）架構盤點
- [x] 價值評估：oracle 評估原生 Rust 實作 Go/No-Go（結論：Go, staged）
- [x] `proposal.md` / `design.md`：定位（雙重角色 monorepo）、效能對比表、工具收斂清單
- [x] 與 graphify-core v1 契約對齊（直接驗證 `graphify-core/src/`）：
      `GraphifyPlugin` trait 簽名、`WorkspaceContext`、`GraphUpdateEvent{modified_nodes}`、
      `GraphOutput` / `Node` / `Edge` 型別、`query_bfs` / `find_shortest_path` / `from_toon` 可用性
- [x] 雙語 README 建立（英文 + 台灣繁體中文）

## Task 1.5: SDK 整合示範文件（雙重角色）
- [x] `sdk/` 目錄：SDK 接入層草稿（Python tool 對接 SDK 協議的初步架構）
- [x] `docs/integration/`：SDK library 整合進原本是不同語言寫的 MCP 的過程紀錄（定位與目錄結構）
- [x] README 補上雙重定位說明與目錄導覽

## Task 2: Crate 骨架
- [ ] `Cargo.toml`：package `graphify-plugin-review`，lib `graphify_plugin_review`，serde/thiserror + graphify-core path dep
- [ ] `src/lib.rs`：實作 `GraphifyPlugin` trait（get_id / bind / get_workspace_key / sync_toon）
- [ ] 單元測試：bind 注入 workspace_key 正確性

## Task 3: 核心分析（Phase 1 MVP）
- [ ] `review_impact`：從 `on_graph_updated` 的 `modified_nodes` 取變更節點 → petgraph BFS（depth 2）→ 受影響範圍
- [ ] `review_callers`：反向 `calls` 邊追蹤
- [ ] `review_entrypoints`：無 incoming `calls` 的函數偵測
- [ ] `review_flows`：執行路徑追蹤 + 關鍵度
- [ ] 16ms BFS impact trace 效能預算驗證

## Task 4: Benchmark 驗證
- [ ] 標準開源專案（tokio / rust-analyzer）冷啟動與查詢延遲基準
- [ ] 對比 Python upstream（`legacy/code-review-graph/`）：impact BFS 目標 10–20x、flow trace 10–20x
- [ ] 記憶體：目標減少 50–70%（消除 SQLite 與重複圖儲存）
- [ ] 工具響應：95% 查詢 < 10ms

## Task 5: 收尾
- [ ] 雙語文件同步（功能、設定、授權一致）
- [ ] 開源去識別檢查（無私有主機名 / 本地路徑）
- [ ] commit + push

## [待討論] 未決項目
- Embeddings 語義搜尋：待 GraphifyRust RAG boundary 決策（llm 層 vs plugin sidecar）
- Community detection：僅在真實需求出現時實作
- 上游 Python `code-review-graph` 向後相容橋接 → 同 handoff 決議：無 MCP 註冊需求即無橋接必要
- SDK 接入層（sdk/）：SDK 本身尚未開發（GraphifyRust Layer 2 暫緩），草稿僅為架構示範
