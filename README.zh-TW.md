# graphify-plugin-review

**雙軌平行 + 漸進式演進 (Dual-Track Evolutionary) monorepo** — 以
`code-review-graph`（Python，tirth8205）為單一起點，展示兩條平行的進化路徑：

**軌 A — 原生 Rust rewrite**（`crates/`）：第一方、內嵌型 Rust crate
（實作 `GraphifyPlugin` trait），提供結構化 Code Review 分析 — 衝擊半徑、
呼叫者、進入點、執行流程 — 直接運行於 Graphify Core 記憶體內的 petgraph。
所有 `review*` 工具由 GraphifyMCP 啟動時自動註冊。
目標：16ms 零開銷、單一二進位、記憶體級 git diff 拓撲解析
（經由 proposed 的 `analyze_diff_impact` trait 方法）。

**軌 B — Graphify Python SDK + 改版 MCP**（`graphify-sdk-python` + `python/review-mcp/`）：
`graphify-sdk` 是 Graphify 官方對外（Python 生態圈）的打底基礎設施（高階
async API、Stdio/JSON-RPC 封裝、`workspace_key` 透傳）。改版 Python Review MCP
是它的**第一個 first-class client**：100% 沿用成熟 Python Review Skills，並透過
注入 `get_blast_radius(git_diff)` 取得的 `{{ topology_impact_toon }}`，
升級成「拓撲感知審查 (Topology-Aware Review)」。

> **狀態**：文件先行階段。分析目標與架構已載於 `openspec/`；crate 骨架待
> graphify-core v1 契約對齊（已直接驗證 `graphify-core/src/`）。

## 目錄結構

```
├── crates/
│   └── graphify-plugin-review/   # [軌 A] 原生 Rust rewrite（內嵌 GraphifyPlugin）
├── sdk/                          # pointer → 官方 repo graphify-sdk-python
├── python/
│   └── review-mcp/               # [軌 B] 改版 Python Review MCP（SDK first-class client）
├── legacy/code-review-graph/     # 原始 Python tool（fork, reference）
├── docs/integration/             # SDK 整合進不同語言 MCP 的過程紀錄
└── openspec/                     # proposal / design / tasks（本變更）
```

## 為什麼原生（軌 A）

上游 `code-review-graph`（Python，[tirth8205](https://github.com/tirth8205/code-review-graph)，MIT）驗證了
Review 工作流程，但作為獨立程序它重複 Graphify 的圖、持自有 SQLite store、
且附帶 30+ 工具。原生 plugin：

- **複用記憶體圖** — 分析直接運行於 Graphify 的 petgraph（`GraphOutput`）；
  無 SQLite 鏡像、無重複解析管線。
- **消除啟動成本** — 內嵌 crate，無 interpreter 啟動、無 30 MB 依賴載入。
- **收斂工具面** — 精簡的 `review*` 工具（impact / callers / entrypoints /
  flows），不是 30 個工具。
- **Zero mock** — 每個分析都跑在真實記憶體圖上；變更偵測來自 Graphify Core
  的 `on_graph_updated`（`modified_nodes`），不是重寫一套 git diff。

### 效能目標（對比 Python 上游）

| 操作 | Python (ms) | 原生目標 (ms) | 增益 |
|-----------|------------|--------------------|------|
| impact BFS (1000 nodes) | 50–200 | 2–10 | 10–20x |
| flow trace | 20–100 | 2–5 | 10–20x |
| symbol search (10k nodes) | 100–500 | 5–20 | 5–20x |
| AST parse per file | 100–300 | 80–250 | 1.2–1.5x |

## 為什麼 SDK + 改版 MCP（軌 B）

- `GraphifyClient(workspace_key)` 自動處理與 graphify-mcp 的 Stdio/JSON-RPC
  通訊與進程生命週期 — Python 開發者 / Agent 零樣板。
- `get_blast_radius(git_diff, depth)` / `query_symbol_topology(symbol_name)`
  以 16ms（core 計算）拉取 `.toon` 壓縮拓撲。
- 改版 MCP 保留 100% 成熟 Python 資產（Skills、Prompt 範本、安全與效能檢查），
  加上拓撲感知審查：LLM 能精準警告「2 階以外的 breaking change risk」，
  而不只是看行級別 diff。

## 生態對齊

- **Graphify Plugins 一員**：與 `graphify-plugin-handoff`、
  `graphify-plugin-opendoc` 平行；plugin 之間以 `workspace_key`（graphify-core v1
  契約）對齊，不各自 walk-up。
- **契約**：實作 `GraphifyPlugin`（`get_id` / `bind` /
  `get_workspace_key` / `sync_toon` / `on_graph_updated`）；傳輸與工具註冊由
  graphify-mcp 負責。`analyze_diff_impact` 為 proposed forward contract，
  屬 graphify-core 領域（見 `openspec/`）。
- **開源安全**：版本控制檔案無私有主機名、本地 IP、或本機路徑。

## 開發

- 軌 A（Rust）：`cargo build` / `cargo check` / `cargo clippy` / `cargo test`
- 軌 B（Python）：SDK 位於官方 repo `graphify-sdk-python`；整合流程見
  `docs/integration/`。

## 參考

- 上游 Python 原始碼以 fork 存放於 `legacy/code-review-graph/`
  （參考材料，已移除 `.git` — 納入追蹤，供 SDK 整合示範使用）。
- 完整架構與任務拆解：`openspec/changes/code-review-native-plugin/`。
