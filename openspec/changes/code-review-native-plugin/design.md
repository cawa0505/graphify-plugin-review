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

> `workspace_key` 欄位僅作為 CRG 端 provenance 標記；plugin 端綁定一律以
> 當前 bound 的 `GraphifyPlugin::get_workspace_key()` 為範圍（與 relay /
> opendoc 一致），不採用 payload 內的值。

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

### 4.2 review_bindings 表（graphify.db 內，Slice 0 已 shipped 原樣）

```sql
CREATE TABLE IF NOT EXISTS review_bindings (
    workspace_key     TEXT NOT NULL,     -- plugin 當前 bound 的 workspace_key
    id                TEXT NOT NULL,    -- CRG review_id（IngestPayload 內）
    canonical_node_id TEXT NOT NULL,    -- GraphOutput Node.id 原樣：{file}:{kind}:{name}
    file_path         TEXT NOT NULL,    -- 原始 IngestPayload 行號所屬檔
    line_number       INTEGER NOT NULL, -- 綁定時之行號（僅記錄用，不參與查詢鍵）
    signature_hash    TEXT NOT NULL,    -- 預留：Slice 1+ 結構漂移偵測
    severity          TEXT NOT NULL,    -- critical | high | medium | low | info
    category          TEXT NOT NULL,    -- security | performance | correctness | style
    comment           TEXT NOT NULL,    -- CRG 原評語
    status            TEXT NOT NULL DEFAULT 'unresolved', -- unresolved|resolved|dismissed
    created_at        TEXT NOT NULL,    -- RFC 3339
    updated_at        TEXT NOT NULL,   -- RFC 3339
    PRIMARY KEY (workspace_key, id)
);

CREATE INDEX IF NOT EXISTS idx_review_node
    ON review_bindings (workspace_key, canonical_node_id);
CREATE INDEX IF NOT EXISTS idx_review_status
    ON review_bindings (workspace_key, status);
```

> 裁決 R4：SQLite 併入既有 graphify.db（與 opendoc 同款 pattern — plugin
> 對同一檔開自己的 `rusqlite::Connection` + `CREATE TABLE IF NOT EXISTS`），
> 不單獨開檔，避免檔案鎖衝突與多 DB 管理成本。
>
> workspace_key 範圍規則（與 relay / opendoc 一致）：bindings 一律以此 plugin
> 當前 bound 的 `GraphifyPlugin::get_workspace_key()` 為查詢範圍；
> `IngestPayload.workspace_key`（CRG 端 provenance）只記錄於原始 `source`
> 而不寫入此表 — Slice 0 已修正更行（commit `69fa8bb`）。

## 5. Canonical Node ID 語意（Slice 0 已修訂）

- `canonical_node_id` = **Graphify extract 實際產生的 `Node.id`** 格式為
  `{file_path}:{kind}:{name}`（如 `src/auth.rs:function:verify_token`），
  **不是**早期 spec 假設的 `crate::auth::verify` 點號形式。
- 內部 `NodeId(String)` 是 graphify-core 唯一的節點識別字串，**會隨 extract
  設定而帶 `./` 前綴**（相對路徑執行時）— Slice 0 已對齊：
  `resolver.rs` 回傳 `Node.id` 原樣，使本 plugin 與
  `GraphUpdateEvent.modified_nodes` 共用同一 namespace
  （orbit memory #3515 / #3516 確認）。
- `resolver.rs` 的線掃規則：在當前快取的 GraphOutput 中，找 `source_file`
  相符且 `start_line <= line <= end_line` 的節點；多重命中取 **最內層**
  （span 最小者）；找不到 → binding `canonical_node_id` 存空字串
  （`""` 表示 orphan，仍寫入 review_bindings 供 `review_get_context("")`
  查回）。
- 行號小幅位移（不影響 Node.id）→ 綁定查詢仍命中
  （這正是「行號脆弱性」解法的核心）；Node.id **改變**（rename / 換檔 /
  換 kind）→ 舊 binding 變成 orphan — Slice 1 `on_graph_updated`
  auto-resolver 利用此訊號判定「問題已不存在」。

## 6. MCP 介面（經 graphify-mcp 自動註冊，Slice 0 已 shipped）

| Tool | 型態 | 輸入 | 輸出 |
|------|------|------|------|
| `reviewIngest` | Mutating | `{path: string}` | `{bound_count, orphan_lines_count}` |
| `reviewGetContext` | Read | `{canonical_node_id: string}` | `{unresolved: [{review_id, severity, category, comment, file_path, line_number}]}` |
| `reviewResolve` | Mutating | `{review_id: string, reason?: string}` | `{resolved: review_id, status}` |

> 裁決 R2：plugin 不實作 MCP server；工具由 graphify-mcp 於啟動時
> auto-register（handoff `relay*` / opendoc `opendoc*` 同款）。
>
> graphify-mcp 對 `reviewIngest` 的特殊接手：呼叫 plugin 前，
> 先從 workspace 圖快取（`GraphState.graph_data`）取得 `GraphOutput`
> 並 `sync_toon(Some(toon_bytes))` 餵給 plugin，使 resolver 有圖可線掃。
> （此 graph-feed 抽手在 GraphifyRust `424cd72` 已 shipped。）

## 7. Slice 1 — Drift Guard & Auto-Resolution（本批細部規格）

### 7.1 觸發模型

核心觀察：**Node.id 是綁定鍵；GraphOutput 在 `sync_toon` 時被整個替換**。
因此每次 `on_graph_updated(event)` 收到時，plugin 已持有「事件過後的」新
GraphOutput 快取。Auto-resolver 只需做：

> 對此 `workspace_key` 底下所有 `status='unresolved'` 的 binding，
> 若其 `canonical_node_id` 不存在於當前快取 GraphOutput 的節點集合
> (不含空字串 orphan — orphan 保留待人工) → 標記 `status='resolved'`、
> `resolution_reason='auto: node gone'`、`updated_at = now()`。

此邏輯**完全不依賴 `event.modified_nodes`** — 更穩健（modified_nodes 只列
變動者，不列刪除者；依賴它會漏判消失節點）。

### 7.2 Signature Hash：YAGNI 提案 [待討論]

原 spec 將 `signature_hash`（「AST 節點結構 hash」）作為 Slice 1 T1.1
標配，意圖偵測「節點 id 未變但內部結構已變」。實作上顯著的問題：

1. graphify-core v1 的 `GraphifyPlugin` trait **不暴露 AST handle**，只剩
   `sync_toon` 收到的 `GraphOutput` 欄位（`id` / `label` / `kind` /
   `source_file` / `start_line` / `end_line` / `doc_comment` / `description` /
   `metadata`）。除掉會 cosmetic-flip 的（`doc_comment`、`description`、
   `metadata`）與會自然漂移的（`start_line` / `end_line`），可算「結構」僅剩
   `{label, kind, file_type, language}` — 但任何這幾個改動都連帶改 `Node.id`
   （file/kind/name 是 Node.id 的格構成元），不會出現「id 不變但結構變」的
   場景。
2. 即便能算出有效 hash，「hash 變但節點還在」對應的是「同一符號內容重構」
   — review 註解通常**仍然適用**（如「verify_token 中 use constant-time
   compare」不會因內部重構而失效）。標 `drifted` 待人工與直接靜默不動
   幾乎沒區別。

**[已裁決 — YAGNI 砍比對實作]**：Slice 1 砍 `signature_hash` 的「比對實作」，
僅保留欄位（schema 已 ready，無成本），寫入固定預設值 `v1_default`。
Node 消失即自動銷案（`resolved_by='auto:node_gone'`），覆蓋 99% 漂移場景。
真如有 rename + body-preserving + review 該轉移的個案需求浮現再回頭加。

### 7.3 review_resolve 完整化

- Slice 0 已交付手動 `review_resolve`（CLI / MCP 都打通）。
- Slice 1 補上 local DB 內 `resolution_reason` 欄（schema 變更：
  `ALTER TABLE … ADD COLUMN resolution_reason TEXT`）+ `resolved_at`。
- **CRG 端反向銷案（`crg_client.resolve_review`）與 v1 host 解耦**：
  本 plugin 自動銷案的訊號只在本 plugin 的 graphify.db 內變更；
  CRG 端要同步銷，需 CRG 真有開出 `resolve_review` MCP 工具（見 §9
  CRG RFC）。Slice 1 不阻塞 — 即便 CRG 端沒接，plugin 那邊也呈現 resolved
  狀態，Agent 查詢就收得到「已處理」訊號。

### 7.4 驗收準則（Slice 1）

- [x] `on_graph_updated` 實作：給定 fixture {binding_to_node_A, unrelated_node_B_to_review} → 將 A 自 graph 移除後 sync_toon + on_graph_updated → A 的 binding 自動 resolved（`auto:node_gone`），unrelated 仍 unresolved。**e2e 驗證通過**（CLI fixture：r-101 auto-resolve ✓ / r-102 保持 unresolved ✓）。
- [x] schema migration 不破壞已 shipped bindings（`migrate_v1_1` 用 `PRAGMA table_info` 檢查後 `ALTER TABLE`，舊 schema 上存在資料時可用；migration idempotency 測試通過）。
- [x] 不引入任何網路依賴（Slice 1 無新 dep；`ureq` 仍為 Slice 0 的 crg_client 骨架，預設 NoOp）。
- [x] graphify-mcp 3 tool 行為與 Slice 0 binary-兼容；`resolved_by` / `resolution_reason` 欄位在 response 內 optional，舊 client 透明。

## 8. Slice 2 — Real-time Impact Guard（本批細部規格）

### 8.1 衝擊半徑 BFS 公式

- 觸發：`on_graph_updated(event)` 中 `event.modified_nodes` 非空。
- BFS 種子 = `event.modified_nodes`（實際變動節點）。
- 在 plugin 自己的快取 GraphOutput 上用 graphify-core `query_bfs` 以種子為根、
  `max_depth = 2`（預設；與 `code-review-graph` 的
  `detect_changes_tool` 預設對齊）走逆向邊（往 upstream callers）。
- 注意：現在 plugin 持有 `GraphOutput`（plain nodes/edges Vec）並非 `DiGraph`
  — 必須 `GraphOutput → DiGraph` 轉換 + `find_shortest_path` /
  query_bfs。**[探勘已完成]** graphify-core **無**公開 `GraphOutput → DiGraph`
  helper，但 `graphify_core::DiGraph`（petgraph re-export，`lib.rs:25`）為公開
  型別。T2.1 在 plugin 端自寫 `build_impact_graph(&GraphOutput) -> DiGraph<Node, Edge>`
  （~30-50 行），不需新依賴、不需 core 改動。`query_bfs` /
  `find_shortest_path` 入參為 `&DiGraph<Node, Edge>`，自建 DiGraph 即可直接呼叫。

### 8.2 Impact Alert 判定與狀態

- 對 BFS 涵蓋集合內每個 node，查 `review_bindings` 中
  `workspace_key = 此工作區 AND canonical_node_id = node.id AND
  status = 'unresolved' AND severity IN ('critical', 'high')`。
- 命中 → 產 `ImpactAlert` domain event：
  ```
  ImpactAlert {
      workspace_key:   String,
      modified_node:   NodeId,           // 觸發變動的種子
      impacted_node:   NodeId,           // BFS 涵蓋內含 review 的節點
      review_ids:      Vec<String>,      // 命中 review 的 id 清單
      severities:      Vec<String>,      // 對應 severity
      max_severity:    String,           // critical > high > medium ...
      alert_message:   String,           // 產業格式化訊息，供 mcp 轉發
      event_id:        String,           // uuid v4
      generated_at:   String,            // RFC 3339
  }
  ```

### 8.3 接 graphify-mcp 轉發（graphify-mcp 協商點）

graphify-mcp 監聽什麼？v1 trait 不含 `subscribe_impact_alert` 之類的
host-side API。供選方案（待 GraphifyRust 協商）:

- **方案 A**：plugin 在 `on_graph_updated` 內直接 `notify_impact_alert(...)`
  呼叫透過 trait 注入的 callback closure → graphify-mcp 在建構 plugin時注入
  寫入 `mcp_notify_tx` 的 closure，把 ImpactAlert 序列化為 MCP
  `notifications/review/impact_alert` 推送。**需 trait v1.1 加 callback
  欄位或 constructor hook**。
- **方案 B**：plugin 把 ImpactAlert 寫入 graphify.db 或共用 ring buffer；
  graphify-mcp 自己 poll 取出轉發。 高 latency、輪詢成本。
- **方案 C**：v1 trait 新增 `take_impact_alerts(&mut self) -> Vec<ImpactAlert>`
  方法讓 host 每輪 event 後取走。**需 trait v1.1**。

推薦 **方案 A**（最低 latency、與 trait 哲學一致）。**待 GraphifyRust 端
協商 v1.1 變更 [待討論]。**

### 8.4 驗收準則（Slice 2）

- [ ] `on_graph_updated` 中 BFS depth=2，命中 unresolved high/critical
      binding → 產 ImpactAlert；fixture 含 critical upstream caller 觸發
      種子變動時 event 落地。
- [ ] graphify-mcp 端 ImpactAlert → MCP notification 轉發通道打通
      （依 §8.3 選定方案）。
- [ ] 效能：fixture 50 節點 + 5 reviews 全鏈路（sync_toon → on_graph_updated
      → BFS → alerts）< 50ms。
- [ ] 不阻斷：on_graph_updated 中的 BFS 失敗（如 GraphOutput 空白）不丟
      panic，只回 Err + 寫 log 一行（與 Slice 0 「plugin 永不 panic」契約一致）。

## 9. Slice 路線圖（commit 已落地）

### Slice 0 — 基礎單向 Bridge（shipped）

- [x] repo 清理：砍 `legacy/` + `sdk/` + `python/` + `docs/integration/` — `c344dc4`
- [x] docs 重寫（design / proposal / tasks / 雙語 README）— `c344dc4`
- [x] Crate 建立（Cargo.toml + 7 模組 GraphifyPlugin trait 實作）— `c344dc4`
- [x] `registry.rs`：review_bindings DDL + DAO（workspace_key scoped PK）— `c344dc4`
- [x] `ingest.rs`：IngestPayload JSON 解析 — `c344dc4`
- [x] `resolver.rs`：line→symbol innermost span 匹配（回傳 Node.id 原樣含 `./` 前綴）— `c344dc4`
- [x] `sync.rs`：sync_toon → from_toon 全寬容快取 — `c344dc4`
- [x] `lib.rs`：業務 API review_ingest / review_ingest_file /
      review_get_context / review_resolve（workspace_key 範圍規則修正）—
      `c344dc4` + `69fa8bb`
- [x] `crg_client.rs`：MCP-over-HTTP 骨架（ureq，Box<ureq::Error>）— `c344dc4`
- [x] graphify-cli `review` 子指令 + graphify-mcp 3 review\* tools
      auto-register + e2e — GraphifyRust `424cd72`；tasks 同步 `4c5fdc2`
- [x] 33/33 plugin tests + 21/21 mcp tests + clippy clean

### Slice 1 — Drift Guard & Auto-Resolution（待動工，本 spec 已就緒）

- [ ] T1.1 `signature_hash` YAGNI 裁決 + schema migration
      (`ALTER TABLE ADD COLUMN resolution_reason` 和 `resolved_at`)
- [ ] T1.2 `on_graph_updated` auto-resolver：node 消失 → resolved
- [ ] T1.3 `review_resolve` 加 `resolution_reason` 與 `resolved_by`
      欄位回填
- [ ] T1.4 CRG RFC：`search_reviews` / `resolve_review` 需求規格書
      開出（見 `crg-requirements.md`）

### Slice 2 — Real-time Impact Guard（待 graphify-core v1.1 協商）

- [ ] T2.1 BFS 衝擊半徑引擎（graphify-core `GraphOutput → DiGraph`
      helper 允許性確認）
- [ ] T2.2 `ImpactAlert` domain event struct + 生產邏輯
- [ ] T2.3 graphify-mcp ImpactAlert → MCP notification 轉發
      （跟 GraphifyRust 協商 trait v1.1 以選定 §8.3 方案）

## 10. 對 graphify-core v1 契約驗證

- `GraphifyPlugin` trait：`get_id` / `bind` / `get_workspace_key` /
  `sync_toon` / `on_graph_updated`（已驗證 graphify-core/src/plugin.rs）。
- v1 不暴露 graph handle：plugin 透過 `sync_toon(Some(toon))` 收圖，
  自行 `from_toon` 取得 GraphOutput（from_toon 為 graphify-core root re-export）。
- `Node` 欄位：`id` / `label` / `kind` / `source_file` / `start_line` /
  `end_line`（types.rs）— line→symbol 解析可行。`doc_comment` / `description`
  / `metadata` 為 cosmetic，不宜用於 signature_hash（§7.2）。
- `query_bfs` / `find_shortest_path` 為 graphify-core 公開 primitive
  （`graphify-core/src/graph/query.rs` / `path.rs`）— Slice 2 衝擊半徑 BFS
  採 plugin 端自建 `DiGraph<Node, Edge>`（`graphify_core::DiGraph` re-export）
  路徑， graphify-core 無 GraphOutput→DiGraph helper。
- `GraphUpdateEvent.modified_nodes` 為 `Vec<NodeId>` — Slice 2 種子來源。
  Slice 1 不依賴此欄（採 node presence diff 路徑）。

## 11. 已知限制 / [待討論]

- CRG MCP 目前無 `search_reviews` / `resolve_review` 工具（probe 實測僅 4
  tools：query_graph_tool / detect_changes_tool / review_context_tool /
  minimal_context_tool）— Slice 2 雙向銷案需 CRG 端開發，T1.4 開規格書
  （`crg-requirements.md` 進行中）。
- `signature_hash` 比對 YAGNI 裁決（§7.2）— **已裁決砍比對實作**，
  schema 欄保留寫入 `v1_default`，Slice 1 採 Node.id presence diff。
- Slice 2 ImpactAlert 經 graphify-mcp 轉發的 trait 延伸需求（§8.3
  方案選定）— **已裁決方案 A**（trait v1.1 注入 notify callback
  closure）；Slice 1 驗收通過後再協商 GraphifyRust 端 v1.1 延伸與開工
  Slice 2。
- `GraphOutput → DiGraph` helper 在 graphify-core 的公開程度
  — **已探勘**：graphify-core 無公開 helper；plugin 端自寫
  `build_impact_graph`（~30-50 行），複用 `graphify_core::DiGraph`
  re-export，不需新增依賴。
- `include_impact_radius` 在 `review_get_context` 的 MCP 參數語意需與
  graphify-core `query_bfs` depth 參數對齊。
