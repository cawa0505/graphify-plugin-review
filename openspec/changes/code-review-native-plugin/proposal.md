# Proposal — graphify-plugin-review（Symbol-Native Review Bridge）

## Executive Summary

`graphify-plugin-review` 是 Graphify 專用的 **Review 橋接器 (Symbol-Native
Review Bridge)**：以 `code-review-graph`（CRG）為單一 Review 資料源，將
CRG 產出的結構化 Review 點位（`file_path` + `line_number`）透過 Graphify
Core AST 圖譜 **0ms 升維對齊**至 canonical symbol（Symbol Path，如
`crate::auth::verify`），並託管於本地 `graphify.db`。當程式碼改動觸及
高風險審查點位時，透過 `graphify-mcp` 主動廣播警示與自動銷案，
形成完整 Review 防禦閉環。

本 plugin **不重造 Code Review 引擎** — 它是純 bridge。

## Problem Statement

- **行號脆弱性 (Line-based Fragility)**：傳統 Review 工具僅記錄
  `file_path + line_number`，重構或增刪行數後點位立即失效。
- **重構開銷**：在 plugin 內重新用 Rust 實作完整 Review 引擎開銷巨大；
  Agent 現場執行重型 Review 導致 context 污染與延遲暴增。
- **資訊孤島**：Agent 重構時缺乏對歷史 Review Warnings 的即時感知，
  容易重複犯下曾被標記的 Security / Correctness 錯誤。

## 方向變更記錄

- 先前雙軌方案（Track A native rewrite + Track B Python SDK）已廢止。
- `legacy/`（Python fork）、`sdk/`、`python/`、`docs/integration/` 已刪除。
- Python SDK 發展移至獨立 repo（graphify-sdk-python），不在本 repo 討論。

## Proposed Solution：B + A 混合模式（Slice 0 零阻礙發行）

### 選項 B — File-based Import（Slice 0 主路徑，100% 確定性）

- `review_ingest` 讀取標準 `IngestPayload` JSON 檔案（CRG 導出格式）。
- 與 opendoc Layer 1 同哲學：離線落盤、零外部依賴、零 CRG 介面風險。
- Slice 0 核心能力（line-to-symbol 升維、review_bindings 寫入、.toon
  拓撲合成、review_get_context 查詢）立即 100% 閉環。

### 選項 A — CRG MCP Analysis 接入（crg_client.rs 骨架）

- `crg_client.rs` 實作 MCP Handshake（Rust 說 MCP Protocol），作為即時
  分析工具的調用骨架（對接 CRG 現有 4 tools）。
- Slice 0 預留介面，不阻擋主路徑。

### 選項 C — CRG 規格提案（Slice 1/2）

- `search_reviews` / `resolve_review` 整理為 CRG RFC / Feature Request
  開出，等待外部 CRG 社群或下一階段疊代 — 完全不阻擋當前進度。

## Key Decisions（裁決紀錄）

| # | 裁決 | 內容 |
|---|------|------|
| R1 | canonical_node_id | = Symbol Qualified Name（`crate::auth::verify`），穩定外鍵；內部 hash NodeId 僅為圖內指針，不作綁定鍵 |
| R2 | MCP 歸屬 | plugin 不寫 MCP Protocol Server；review* 工具由 graphify-mcp 自動註冊；ImpactAlert 為 domain event 由 graphify-mcp 轉發 |
| R3 | Sampling | 砍除（反向 LLM 採樣過度複雜，回歸確定性映射 + drift resolution） |
| R4 | SQLite | 併入專案共用 graphify.db（review_bindings 表），不單獨開檔 |
| R5 | Bridge 優先 | 純 bridge 不重造引擎；legacy Python fork 已刪除 |

## Success Criteria

- Slice 0：`review_ingest`（file-based）→ resolver 升維 → `review_bindings`
  寫入 → `review_get_context` 查詢，全鏈路 100% 本地閉環、測試覆蓋。
- graphify-mcp 啟動時自動註冊 3 個 review* tools。
- 零網絡依賴（Slice 0）、零 mock、確定性輸出。
- 開源安全：版本控制無私有主機名、本地 IP、本機路徑。
