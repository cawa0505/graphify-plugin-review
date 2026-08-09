# Design — graphify-plugin-review（Symbol-Native Review Bridge）

> 狀態：**B+A 混合模式已鎖定**（2026-08-10）。
> 方向變更記錄：雙軌 native rewrite（Track A/B）已廢止 — 改為純 bridge；
> `legacy/` Python fork、`sdk/`、`python/`、`docs/integration/` 已刪除。
> 本文檔為唯一權威設計基礎（openspec design.md）。

## 1. 定位

`graphify-plugin-review` 是 Graphify 生態的**語意點位審查橋接器**
（Symbol-Native Review Bridge）：以 `code-review-graph`（CRG）為 Review
資料源，把 CRG 產出的結構化 Review 數據（`file_path` + `line_number`）
透過 Graphify Core 的 AST 圖譜升維對齊至穩定的 canonical symbol
（Symbol Qualified Name，如 `crate::auth::verify`）。

- **不重造引擎**：不實作任何 LLM review 管線、不安裝 GitHub/GitLab SDK、
  不發送線上 API 請求、不修改 Core AST 圖譜。
- **純 bridge**：Review 資料 100% 來自 CRG；本 plugin 負責「升維綁定 +
  本地持久化 + 事件響應」。

## 2. 架構

```
┌───────────────────────────────────────────────────────────────┐
│ code-review-graph (外部 Review 知識源)                          │
│  ├─ 歷史 Review 數據導出檔 (IngestPayload JSON)  ← Slice 0 主路徑 │
│  └─ MCP Server (4 tools: query_graph / detect_changes /          │
│      review_context / minimal_context)          ← crg_client.rs │
└──────────────────────────────┬────────────────────────────────┘
                               │
┌──────────────────────────────▼────────────────────────────────┐
│ graphify-plugin-review (Rust GraphifyPlugin)                   │
│  ├─ ingest.rs     # File-based JSON import + 轉譯               │
│  ├─ crg_client.rs # MCP Client 骨架（Rust 說 MCP Protocol）      │
│  ├─ resolver.rs   # Line-to-Symbol Resolver（對齊 GraphOutput） │
│  ├─ registry.rs   # review_bindings DAO（併入 graphify.db）     │
│  ├─ review.rs     # review_ingest / review_get_context / resolve│
│  └─ sync.rs       # sync_toon 快取 + .toon 上下文合成            │
└──────────────┬─────────────────────────────────▲───────────────┘
               │ 1. Auto-register review* tools │ 2. Dispatch ImpactAlert
┌──────────────▼─────────────────────────────────┴───────────────┐
│ graphify-mcp (MCP Gateway)                                      │
│  - 自動註冊 review_ingest / review_get_context / review_resolve │
│  - 轉發 notifications/review/impact_alert 給 Agent              │
└─────────────────────────────────────────────────────────────────┘
```

## 3. 鋼鐵邊界（Scope）

### 3.1 In-Scope

1. **CRG 數據讀取**：解析 CRG 導出的結構化 Review JSON（IngestPayload 1.0）。
2. **Symbol Mapping**：將 `file_path + line_number` 轉換為 canonical symbol。
3. **SQLite 管理**：`review_bindings` 表（**併入 graphify.db**，非獨立檔案），
   記錄評語狀態、Severity、結構 hash。
4. **單向 Pull API**：`review_get_context` 查詢指定 symbol（含衝擊半徑）
   未解決的審查警示。
5. **雙向事件響應**（Slice 1/2）：`on_graph_updated` 觸發
   ImpactAlert domain event（由 graphify-mcp 轉發，plugin 不寫 MCP server）。

### 3.2 Out-of-Scope（硬性禁止）

- ⛔ 不實作 LLM Code Review（審查意見完全來自 CRG）。
- ⛔ 不支援其他 Review 工具（只認 code-review-graph 格式）。
- ⛔ 不發送線上 API 請求（不裝 GitHub/GitLab SDK、無 Token/OAuth）。
- ⛔ 不修改 Core AST 圖譜（不把 Review 節點塞進 petgraph）。
- ⛔ 不寫 MCP Protocol Server（transport 由 graphify-mcp 統一處理；
  MCP 工具由 graphify-mcp 啟動時自動註冊）。
- ⛔ 不實作 sampling/createMessage（反向 LLM 採樣已裁決砍除）。

## 4. 資料架構

### 4.1 IngestPayload（CRG 輸入契約，schema 1.0）

```json
{
  "version": "1.0",
  "source": "code-review-graph",
  "workspace_key": "my-app-v1",
  "reviews": [
    {
      "review_id": "crg-sec-001",
      "file_path": "src/auth.rs",
      "line_number": 42,
      "severity": "high",
      "category": "security",
      "comment": "Potential timing attack on HMAC token comparison.",
      "created_at": "2026-08-10T00:00:00Z"
    }
  ]
}
```

### 4.2 review_bindings 表（graphify.db 內）

```sql
CREATE TABLE IF NOT EXISTS review_bindings (
  Id TEXT PRIMARY KEY,             -- CRG review_id
  Canonical_node_id TEXT NOT NULL, -- Symbol Path (crate::auth::verify)
  File_path TEXT NOT NULL,         -- 原始檔案路徑
  Line_number INTEGER NOT NULL,    -- 綁定時之行號
  Signature_hash TEXT NOT NULL,    -- 綁定時該 AST 節點的結構 hash
  Severity TEXT NOT NULL,          -- critical | high | medium | low | info
  Category TEXT NOT NULL,          -- security | performance | correctness | style
  Comment TEXT NOT NULL,
  Status TEXT DEFAULT 'unresolved',-- unresolved | resolved | dismissed
  Created_at TEXT NOT NULL,
  Updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_review_node ON review_bindings(canonical_node_id);
CREATE INDEX IF NOT EXISTS idx_review_status ON review_bindings(status);
```

> 裁決 R4：SQLite 併入既有 graphify.db（opendoc 同款 pattern — plugin
> 對同一檔案開自己的 rusqlite 連線 + `CREATE TABLE IF NOT EXISTS`），
> 不單獨開檔，避免檔案鎖衝突與多 DB 管理成本。

## 5. Canonical Node ID 語意（裁決 R1）

- `Canonical_node_id` = **Symbol Qualified Name**（如 `crate::auth::verify`），
  是穩定且人可讀的外鍵。
- Graphify 內部的純整數/hash NodeId 是圖譜內部指針，會隨重新 parsing
  變動 — **不作為綁定鍵**。
- 查詢時，`resolver.rs` 線掃 GraphOutput，把 `file_path + line_number`
  映射到當前 AST 節點的 canonical symbol — AST 重建或行號小幅位移後
  綁定關係依然穩定。
- GraphOutput 由 `sync_toon(Some(toon))` 收到，plugin 用 `from_toon`
  反序列化後快取於記憶體（0ms 本地線掃）。

## 6. MCP 介面（經 graphify-mcp 自動註冊）

| Tool | 型態 | 說明 |
|------|------|------|
| `review_ingest` | Command/Mutating | 載入 CRG JSON 檔案 → 升維綁定 → 寫入 graphify.db。Input: `{path}`。Output: `{success, bound_count, orphan_lines_count}` |
| `review_get_context` | Read-Only | 查詢指定 symbol（含衝擊半徑）未解決評語。Input: `{canonical_node_id, include_impact_radius}` |
| `review_resolve` | Command/Mutating | 標記 review 為 resolved（手動或自動）。Input: `{review_id, resolution_reason}` |

> 裁決 R2：plugin 不實作 MCP server；工具由 graphify-mcp 於啟動時
> auto-register（handoff relay*/opendoc opendoc* 同款）。

## 7. 事件模型（Slice 1/2）

- **ImpactAlert domain event**：`on_graph_updated` 鉤子中比對變動節點與
  `review_bindings` 表；觸及 high/critical 未解決節點時產出
  `ImpactAlert { modified_node_id, impacted_review_node, review_id, severity, alert_message }`。
- **graphify-mcp 轉發**：graphify-mcp 監聽該 event，透過現有 MCP transport
  發送 `notifications/review/impact_alert` 給 client。
- **Auto-Resolution**：偵測到 AST 結構修復（signature_hash 比對）時，
  呼叫 `crg_client.resolve_review()` 雙向銷案。
- **Sampling 已砍除**：反向採樣引入過度雙向複雜度；回歸
  「0ms 線掃確定性映射 + 手動/自動 drift resolution」。

## 8. Slice 路線圖

### Slice 0（當前）— 基礎單向 Bridge（零網絡、100% 確定性）

- [x] repo 清理：砍 `legacy/`（Python fork）、`sdk/`、`python/`、`docs/integration/`
- [x] docs 重寫（本文件 + proposal + tasks + 雙語 README）
- [ ] Crate 建立（Cargo.toml + GraphifyPlugin trait 實作）
- [ ] `registry.rs`：review_bindings DDL + DAO
- [ ] `ingest.rs`：IngestPayload JSON 解析 → 轉譯
- [ ] `resolver.rs`：Line-to-Symbol Resolver（對 GraphOutput 線掃）
- [ ] `sync.rs`：sync_toon 快取 GraphOutput
- [ ] `review.rs`：review_ingest / review_get_context / review_resolve 業務 API
- [ ] `crg_client.rs`：MCP Client 骨架（Rust 說 MCP Protocol，對接 CRG 4 tools）
- [ ] graphify-mcp auto-registration + e2e

### Slice 1 — Drift Guard & Auto-Resolution

- [ ] signature_hash 比對：代碼結構改變時自動檢查修復狀態
- [ ] `on_graph_updated` 自動銷案（呼叫 CRG resolve_review）
- [ ] `review_resolve` 工具鏈完整化

### Slice 2 — Real-time Impact Guard

- [ ] 訂閱 Core AST Event Bus（`on_graph_updated`）
- [ ] 沿變動節點 BFS 衝擊半徑，檢查觸及 high/critical 未解決節點
- [ ] 產出 ImpactAlert domain event → graphify-mcp 轉發
      `notifications/review/impact_alert`

## 9. 對 graphify-core v1 契約驗證

- `GraphifyPlugin` trait：`get_id` / `bind` / `get_workspace_key` /
  `sync_toon` / `on_graph_updated`（已驗證 graphify-core/src/plugin.rs）。
- v1 不暴露 graph handle：plugin 透過 `sync_toon(Some(toon))` 收圖，
  自行 `from_toon` 取得 GraphOutput。
- `Node` 欄位：`id` / `label` / `source_file` / `start_line` / `end_line`
  （types.rs）— line→symbol 解析由此可行。
- `query_bfs` / `find_shortest_path` 為 graphify-core 公開 primitive
  （graph/query.rs、graph/path.rs）— Slice 2 衝擊半徑 BFS 可複用。
- 無 line→node resolver API 存在於 core — resolver 由 plugin 自行實作
  （對快取的 GraphOutput 線掃）。

## 10. 已知限制 / [待討論]

- CRG MCP 目前無 `search_reviews` / `resolve_review` 工具（probe 實測
  僅 4 tools：query_graph_tool / detect_changes_tool / review_context_tool
  / minimal_context_tool）— 真雙向銷案需 CRG 端開發（Slice 2 開規格）。
- `include_impact_radius` 的 BFS 語意需與 graphify-core `query_bfs` 對齊。
- ImpactAlert 的 graphify-mcp 轉發機制（Slice 2 時與 GraphifyRust 協商）。
