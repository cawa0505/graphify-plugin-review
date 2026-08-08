# Tasks — Code Review Native Plugin

> 文件先行：Task 1 為文件與契約對齊，Task 2 起才動程式碼。
> 阻塞相依：graphify-core v1 `GraphifyPlugin` trait / `GraphOutput` 需存在（GraphifyRust 側，已確認）。
> 本 repo 為**雙軌平行 monorepo（Dual-Track Evolutionary）**：
> 軌 A = 原生 Rust rewrite（`crates/`）；軌 B = graphify-sdk（`sdk/`）+ 改版 Python MCP（`python/review-mcp/`）。
> SDK 落點定案：graphify-sdk 先放 `sdk/`，以可抽出結構建置（API 穩定後可無痛 `git mv` 成獨立 repo）。

## Task 1: 文件與契約對齊（文件先行）
- [x] 上游研究：`legacy/code-review-graph/`（tirth8205/code-review-graph v2.3.6 fork）架構盤點
- [x] 價值評估：oracle 評估原生 Rust 實作 Go/No-Go（結論：Go, staged）
- [x] `proposal.md` / `design.md`：定位（雙軌平行 monorepo）、效能對比表、工具收斂清單、`analyze_diff_impact` forward contract
- [x] 與 graphify-core v1 契約對齊（直接驗證 `graphify-core/src/`）：
      `GraphifyPlugin` trait 簽名、`WorkspaceContext`、`GraphUpdateEvent{modified_nodes}`、
      `GraphOutput` / `Node` / `Edge` 型別、`query_bfs` / `find_shortest_path` / `from_toon` 可用性
- [x] 雙語 README 建立（英文 + 台灣繁體中文）

## Task 1.5: SDK 整合示範文件（雙軌）
- [x] `sdk/` 目錄：graphify-sdk 定位與可抽出結構（pyproject 獨立、模組獨立）
- [x] `docs/integration/`：SDK library 整合進原本是不同語言寫的 MCP 的過程紀錄（定位與目錄結構）
- [x] README 補上雙軌定位說明與目錄導覽

## Task 2: [軌 A] Crate 骨架
- [ ] `Cargo.toml`：package `graphify-plugin-review`，lib `graphify_plugin_review`，serde/thiserror + graphify-core path dep
- [ ] `src/lib.rs`：實作 `GraphifyPlugin` trait（get_id / bind / get_workspace_key / sync_toon / on_graph_updated）
- [ ] `src/impact.rs`：git_diff 符號解析（Struct/Function）→ `modified_nodes` 對映
- [ ] `src/prompt.rs`：git diff + `.toon` 衝擊子圖合併 → Review 上下文（MVP 用 `format!`，不引 Tera/Jinja2）
- [ ] 單元測試：bind 注入 workspace_key 正確性

## Task 3: [軌 A] 核心分析（Phase 1 MVP）
- [ ] `review_impact`：`modified_nodes` → petgraph BFS（depth 2）→ 受影響範圍
- [ ] `review_callers`：反向 `calls` 邊追蹤
- [ ] `review_entrypoints`：無 incoming `calls` 的函數偵測
- [ ] `review_flows`：執行路徑追蹤 + 關鍵度
- [ ] 16ms BFS impact trace 效能預算驗證
- [ ] （待 GraphifyRust 對齊後）`analyze_diff_impact(&self, ctx, git_diff, graph)` trait 方法實作

## Task 4: [軌 B] graphify-sdk（Python）
- [ ] `pyproject.toml`：package `graphify-sdk`（可抽出結構）
- [ ] `graphify_sdk/client.py`：`GraphifyClient(workspace_key)` — Stdio/JSON-RPC 封裝 + 進程生命週期
- [ ] `graphify_sdk/api.py`：`get_blast_radius(git_diff/files, depth=3)` / `query_symbol_topology(symbol_name)`（async）
- [ ] `graphify_sdk/workspace.py`：workspace_key 透傳
- [ ] 單元測試：client 生命週期 + workspace_key 透傳

## Task 5: [軌 B] 改版 Python Review MCP
- [ ] 100% 繼承 Python 資產（Skills / Prompt 範本 / 安全與效能檢查）
- [ ] 接入 graphify-sdk：`review_pull_request(workspace_key, git_diff, modified_files)` 工具
- [ ] `{{ topology_impact_toon }}` 注入 Review Prompt（Breaking Change Risk 警告）
- [ ] 與軌 A benchmark 對比：同 diff 下拓撲感知 vs 行級別審查品質

## Task 6: Benchmark 驗證
- [ ] 標準開源專案（tokio / rust-analyzer）冷啟動與查詢延遲基準
- [ ] 對比 Python upstream（`legacy/code-review-graph/`）：impact BFS 目標 10–20x、flow trace 10–20x
- [ ] 記憶體：目標減少 50–70%（消除 SQLite 與重複圖儲存）
- [ ] 工具響應：95% 查詢 < 10ms

## Task 7: 收尾
- [ ] 雙語文件同步（功能、設定、授權一致）
- [ ] 開源去識別檢查（無私有主機名 / 本地路徑）
- [ ] commit + push

## [待討論] 未決項目
- Embeddings 語義搜尋：待 GraphifyRust RAG boundary 決策（llm 層 vs plugin sidecar）
- Community detection：僅在真實需求出現時實作
- 上游 Python `code-review-graph` 向後相容橋接 → 同 handoff 決議：無 MCP 註冊需求即無橋接必要
- `analyze_diff_impact` trait 方法：屬 GraphifyRust plugin.rs forward contract，未對齊前不實作
- graphify-sdk 是否抽成獨立 repo：API 穩定後再評估（現行 `sdk/` 為可抽出結構）
