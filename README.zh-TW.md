# graphify-plugin-review

以 `code-review-graph`（Python，tirth8205）為單一起點的**雙重角色 monorepo**，展示兩條進化路徑：

1. **原生 Rust rewrite** — 第一方內嵌型 Rust crate（實作 `GraphifyPlugin` trait），
   提供結構化 code review 分析 — 影響半徑、callers、entrypoints、flows — 直接運行於
   Graphify Core 記憶體內的 petgraph 圖。所有 `review*` 工具由 GraphifyMCP 於啟動時自動註冊。
2. **SDK 整合示範** — 展示未來 Graphify SDK（Layer 2）怎麼接入原本以不同語言（Python）
   撰寫的 MCP tool：fork 在 `legacy/code-review-graph/`，SDK 接入層草稿在 `sdk/`，
   整合過程紀錄在 `docs/integration/`。

> **狀態**：文件先行階段。分析目標與架構已定義於 `openspec/`；crate 骨架待
> graphify-core v1 契約對齊後進行。

## 目錄結構

```
├── crates/                  # 原生 Rust rewrite（內嵌 GraphifyPlugin）
├── sdk/                     # SDK 接入層草稿（Python tool ↔ SDK 協議）
├── legacy/code-review-graph/     # 原始 Python tool（fork，參考用）
├── docs/integration/        # SDK 整合進不同語言 MCP 的過程紀錄
└── openspec/                # 本變更的 proposal / design / tasks
```

## 為什麼原生實作

上游 `code-review-graph`（Python，[tirth8205](https://github.com/tirth8205/code-review-graph)，MIT）驗證了
review 工作流程，但作為獨立進程：重複持有 Graphify 的圖、自建 SQLite store、提供 30+ 工具。
原生 plugin 則：

- **直接複用記憶體圖** — 分析運行於 Graphify 的 petgraph（`GraphOutput`）之上；
  無 SQLite 鏡像、無重複的解析管線。
- **消除啟動成本** — 內嵌 crate，無直譯器啟動、無 30 MB 依賴載入。
- **收斂工具集** — 最小 `review*` 介面（impact / callers / entrypoints / flows），
  不是 30 個工具。
- **零 Mock** — 每次分析皆對真實記憶體圖執行；變更偵測來自 Graphify Core 的
  `on_graph_updated`（`modified_nodes`），不再自行實作 git diff。

### 效能目標（對比 Python 上游）

| 操作 | Python (ms) | 原生目標 (ms) | 改善 |
|------|------------|---------------|------|
| impact BFS（1000 節點） | 50–200 | 2–10 | 10–20x |
| flow trace | 20–100 | 2–5 | 10–20x |
| symbol search（10k 節點） | 100–500 | 5–20 | 5–20x |
| 單檔案 AST 解析 | 100–300 | 80–250 | 1.2–1.5x |

## 生態對齊

- **屬於 Graphify Plugins**：與 `graphify-plugin-handoff`、`graphify-plugin-opendoc` 平行；
  plugin 之間以 `workspace_key`（graphify-core v1 契約）對齊 — 不各自 walk-up。
- **契約**：實作 `GraphifyPlugin`（`get_id` / `bind` / `get_workspace_key` / `sync_toon`）；
  transport 與工具註冊由 graphify-mcp 負責。
- **開源安全**：版本控制檔案中不含私有主機名、本地 IP 或機器路徑。

## 開發

- 建置：`cargo build`
- 檢查/Lint：`cargo check` / `cargo clippy`
- 測試：`cargo test`

## 參考

- 上游 Python 原始碼以 fork 形式存放於 `legacy/code-review-graph/`
  （參考材料，已移除 `.git` — 納入追蹤，供 SDK 整合示範使用）。
- 完整架構與任務拆解：`openspec/changes/code-review-native-plugin/`。
