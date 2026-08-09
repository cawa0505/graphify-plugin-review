# CRG MCP 擴充需求規格書 — graphify-plugin-review 對接

> 對接方：code-review-graph (CRG) MCP Server
>
> 需求方：graphify-plugin-review（Graphify 內嵌 Rust plugin）
>
> 狀態：草案（2026-08-10），等 CRG 端回覆實作排程後凍結
>
> 範圍：本文僅定義 **新增** MCP 工具的需求，**不**修改 CRG 現有 4 個工具
> （`query_graph_tool` / `detect_changes_tool` / `get_review_context_tool` /
> `minimal_context_tool` 之 schema）— 向後相容。

## 1. 背景與動機

`graphify-plugin-review` Slice 0 已 shipped：能把 CRG 餵入的 review 點位
（`file_path + line_number`）經 graphify-core AST 升維成
`canonical_node_id`（Graphify Node.id 原樣）後，綁定至 `graphify.db` 的
`review_bindings` 表。Slice 1 / Slice 2 想做：

1. **Slice 1**：CRG 端 review 被 graphify 自動判定已修正後，希望
   **反向通知 CRG 把該 review 同步銷案** — 避免 CRG 持續持有過時的
   unresolved 紀錄。
2. **Slice 2**：graphify plugin 在 BFS 衝擊半徑發現 high/critical
   review 被觸及時，希望 **主動查詢 CRG 拿該 review 的最新狀態 + 人類
   reviewer 的補充評語**，補強 domain context（如：「同一 review_id 在
   CRG 已被 reviewer 送出 comment，可以一起納入 ImpactAlert」）。

為了支援這兩個閉環，CRG 端需新增 2 個 MCP 工具，規格見 §2 / §3。

## 2. R1 — `search_reviews`（Read-Only / Mutating-idempotent）

### 2.1 用途

讓 plugin 依多個查詢準則批次取回 CRG 保管中的 reviews（不只是 local
graphify.db 已綁定的）。Slice 2 ImpactAlert 時補充 context 用。

### 2.2 工具名

`search_reviews`

### 2.3 輸入 schema

```json
{
  "workspace_key": "string  (可選；CRG 端紀錄時可標記所屬工作區)",
  "review_ids":    ["string"] (可選；明確指定要查的 id；給空表示全部)",
  "status":        "string  (可選；預設 'unresolved'",
                   "      其他：'resolved' | 'dismissed' | 'all')",
  "severity":      ["string"] (可選；'critical' | 'high' | 'medium' | 'low' | 'info'",
                   "      採 OR 集合，空表示所有)",
  "category":      ["string"] (可選；'security' | 'performance' | 'correctness' | 'style'",
                   "      採 OR 集合，空表示所有)",
  "file_path":     "string  (可選；限定某檔)",
  "line_range":    { "start_line": int, "end_line": int } (可選)",
  "limit":         "int (1..500，預設 100)",
  "cursor":        "string (可選；CRG 端分頁 token)"
}
```

### 2.4 輸出 schema

```json
{
  "hits": [
    {
      "review_id":   "string",
      "workspace_key": "string | null",
      "file_path":    "string  (相對 repo path，與 IngestPayload 一致)",
      "line_number":  "int (CRG 寫入當下的行號，快照性質)",
      "severity":     "string",
      "category":     "string",
      "comment":      "string",
      "created_at":   "RFC 3339",
      "updated_at":   "RFC 3339",
      "status":       "string ('unresolved' | 'resolved' | 'dismissed')",
      "resolved_by":  "string | null ('manual' | 'auto: <rule>' | null)",
      "reviewer_notes": ["string"] (可選；CRG 人類 reviewer 附加的"
                              "                 註解或對話，無則空陣列)"
    }
  ],
  "next_cursor": "string | null (沒下一頁時 null)",
  "truncated":   "boolean (是否因 limit 截斷)"
}
```

### 2.5 語意與不變量

- **Idempotent**：相同 input 回相同 output（CRG 不在這工具內改動狀態）。
- **向後相容**：CRG 端如尚未實作 `resolved_by` / `reviewer_notes`，回
  `null` / `[]` 即可；plugin 端會 graceful 退避。
- **不返回跨 workspace 的 hits**：當 `workspace_key` 未指定時可返回全部
  （這是 CRG 選擇），但 plugin 端會以綁定的 `workspace_key` 二次過濾
  — CRG **不**被要求強制 workspace 隔離（與現行 4 工具一致）。
- **時間穩定性**：`created_at` 不變；`updated_at` 反映 CRG 端上次狀態變動
  — plugin 端用此欄偵測「CRG 自己改過狀態」衝突情況。

### 2.6 不需要保證的範圍

- **`file_path` 可能與綁定時不同**（rename / 顯式遷移）— plugin 已不在乎
  file_path，只在乎 `review_id` 作 PK。
- **Rate limit / pagination 實作細節**由 CRG 自決，`next_cursor` 可為不透明
  token。

## 3. R2 — `resolve_review`（Mutating / Idempotent）

### 3.1 用途

graphify 自動銷案或人工銷案後，反向通知 CRG 把該 review 的狀態改為
`resolved`，避免雙邊狀態分歧。

### 3.2 工具名

`resolve_review`

### 3.3 輸入 schema

```json
{
  "review_id":         "string (必填)",
  "resolution_reason": "string (必填，自由文字，記錄銷案原因)",
  "resolved_by":       "string (必填；'manual' | 'auto:node_gone' | "
                       "            'auto:signature_match' 或其他自訂前綴)",
  "resolved_at":       "RFC 3339 (可選；未指定時 CRG 採當下時間)",
  "force":             "boolean (預設 false；true 表示即使 CRG 端已是 "
                       "resolved 仍更新 updated_at 與 resolved_by)"
}
```

### 3.4 輸出 schema

```json
{
  "review_id":         "string",
  "status":            "string ('resolved')",
  "updated_at":        "RFC 3339 (CRG 端實際更新時間)",
  "previous_status":   "string (銷案前的 CRG 端狀態)",
  "conflict_note":     "string | null (若 previous_status 不是 'unresolved'",
                       "                   回短說明；正常銷案則回 null)"
}
```

### 3.5 語意與不變量

- **Idempotent**：對同一 `review_id` 重複呼叫 `resolve_review`，第二次起
  `previous_status` 為 `'resolved'`、`conflict_note` 不為 null 說明
  「已銷案」。`force=false` 時不覆寫 `updated_at`；`force=true` 時才覆蓋
  （plugin 端預設 `force=false`，只在手動延後修補時用 `force=true`）。
- **接受 `review_id` 不存在的情況**：`previous_status = 'not_found'`、
  `status = 'not_found'`、`conflict_note = 'review_id unknown to CRG'`。
  Plugin 把此回應視為「外部早已清理」 — 視為銷案成功，不報錯。
- **`resolved_by` 字串 namespace**：保留 `auto:*` prefix 給未來 plugin 端
  各種 auto-resolver 變體；CRG 不需要解析內容，僅原樣保存 + 在
  `search_reviews` 回應中返回。

### 3.6 不需要保證的範圍

- **不在 CRG 端**做 graph 對齊或 symbol 升維（那是 graphify plugin 的職責）。
- **不要求 CRG 做 broadcast**：CRG 只是狀態持有者，通知下游（agent/UI）
  的工作由 graphify-mcp 的 ImpactAlert 通道處理。

## 4. 與 CRG 現有 4 工具的關係

| CRG 現有工具 | 與 R1/R2 的關係 |
|--------------|-----------------|
| `query_graph_tool` | 無衝突。plugin 不依賴此 (有自己的 graphify-core 圖) |
| `detect_changes_tool` | 無衝突。plugin 端 `on_graph_updated` 改用 graphify 自己的 BFS |
| `get_review_context_tool` | R1 是其 **批次化、結構化** 伸展版；R1 上線後現有工具可保留或降級為 alias |
| `minimal_context_tool` | 無衝突。 |

## 5. 安全性與速率限制

- 兩個工具皆不改 CRG 現有 auth 模型，沿用 CRG server 現有 transport 的
  認證（MCP 通常本機 stdio，免 token；若 CRG 採 HTTP 上線則補帶 API key）。
- 速率限制由 CRG 自決；plugin 端會：
  - `search_reviews`：對每個 `workspace_key` 至多每 30 秒 1 次主動查詢（rate
    limit 在 plugin 端）；被 ImpactAlert 命中時才觸發。
  - `resolve_review`：在 mutation 流上加 `idempotency_key`（plugin 自產
    `uuid_v4`）防網路 retry 造成 double-write — CRG 端應保留 `idempotency_key`
    24h，重複就回既有結果不重跑副作用。

## 6. 待討論項目

1. **`reviewer_notes` 的來源**：CRG 目前是否存在人類 reviewer 評語資料
   源？若不存在，R1 暫不要求此欄，plugin 端路徑不依賴它。
2. **`idempotency_key` 是否進入 R2 schema 必填**：plugin 端建議為必填，
   但若 CRG 端不想做 24h dedup，可降為 optional。〔待 CRG 端回意願〕
3. **`workspace_key` 的 source of truth**：CRG 端寫 review 時若有這個
   欄，用 CRG 自己的 workspace 概念，還是直接抄 IngestPayload 中的字串？
   （graphify 端是抄 IngestPayload 的，純 provenance；不強制 CRG 跟。）
4. **批次上界的取捨**：`limit <= 500` 為預設上界 — CRG 端若負載敏感可降。