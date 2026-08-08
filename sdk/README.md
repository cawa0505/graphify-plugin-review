# sdk/ — Graphify Python SDK (`graphify-sdk`)

> **[軌 B]** Graphify 官方對外（Python 生態圈）的打底基礎設施。
> **狀態：已定案落點** — SDK 語言順序 supersede D5：Python 優先。
> 本目錄以**可抽出結構**建置（pyproject 與模組獨立）；API 穩定後 `git mv` 即可
> 無痛抽成獨立 repo（`graphify-sdk`），現行先與 first-class client 同場演進。

## 定位

- 提供高階 Async API，負責跨進程 / Stdio 封裝與 `workspace_key` 透傳
- 讓 Python 開發者 / Agent 輕鬆拉取 Graphify 的 `.toon` 拓撲
- 與 graphify-mcp（GraphifyRust Main Repo）的 Stdio/JSON-RPC 通訊與進程生命週期由
  `GraphifyClient` 自動處理 — Zero-Boilerplate

## 規劃模組（Task 4）

```
sdk/
├── pyproject.toml            # package: graphify-sdk（可抽出結構）
└── graphify_sdk/
    ├── client.py             # GraphifyClient(workspace_key) — Stdio/JSON-RPC 封裝 + 進程生命週期
    ├── api.py                # get_blast_radius(git_diff/files, depth=3) / query_symbol_topology(symbol_name)（async）
    └── workspace.py          # workspace_key 透傳（自動，免手動 Schema 轉譯）
```

### 高階 API 草稿

```python
client = GraphifyClient(workspace_key)

# 傳入變更程式碼，索取經 .toon 壓縮的衝擊半徑拓撲
topology = await client.get_blast_radius(git_diff, depth=3)

# 給定 Symbol 名稱，查詢其上下游呼叫鏈拓撲
callers = await client.query_symbol_topology("handle_request")
```

## 第一個 First-class Client

改版 Python Code Review MCP（`../python/review-mcp/`）是 graphify-sdk 的
**實戰練兵場**：拿原本成熟的 Python MCP 引入 SDK，用最小成本把 Review Skills
升級成「拓撲感知審查 (Topology-Aware Review)」— 詳見 `../docs/integration/`。

## 與原生 rewrite 的關係（軌 A）

- 兩軌共用同一組語意工具面：`review_impact` / `review_callers` /
  `review_entrypoints` / `review_flows`
- 效能評測（Task 6）以 `legacy/`（Python）為基準，對比 `crates/`（Rust 原生）
- 軌 B 是第三種對照：Python tool **不重寫**、透過 SDK 協議接入 — 證明拓撲能力
  可以輕鬆賦能既有 Python MCP 生態

## 待辦

- [ ] Task 4：pyproject + client.py + api.py + workspace.py + 單元測試
- [ ] [待討論] API 穩定後評估是否抽成獨立 repo（現行可抽出結構）
