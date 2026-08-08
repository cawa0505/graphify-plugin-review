# docs/integration/ — SDK 整合紀錄

> 本目錄紀錄「怎麼把 SDK library 整合進原本是不同語言寫的 MCP 裡」的過程
> 與結論 — 以 `legacy/code-review-graph/`（Python）為具體案例。
> **狀態：[軌 B]** 改版 Python Review MCP（`../python/review-mcp/`）是
> graphify-sdk 的第一個 first-class client — 本目錄即為該整合的 walkthrough。

## 戰略定位（Dual-Track）

```
原始 Python Code Review MCP (legacy/code-review-graph/)
├── [軌 B] 改版：引入 graphify-sdk → 拓撲感知審查（python/review-mcp/）← 本目錄紀錄
└── [軌 A] 重寫：原生 Rust crate → 16ms 極致效能（crates/）
```

軌 B 的價值主張：**用最小成本把成熟的 Python Review Skills 升級成
「拓撲感知審查 (Topology-Aware Review)」** — 不重寫、不 fork 重做，
100% 繼承既有 Python 資產，只加一層 SDK 呼叫。

## 整合內容（規劃）

- [ ] 整合 walkthrough：`python/review-mcp/` 收到 PR / git diff 請求 →
      透過 graphify-sdk 發 `get_blast_radius(git_diff)` → 取得 Graphify 16ms 算出的
      `.toon` 拓撲地圖 → 注入 Review Prompt
- [ ] `{{ topology_impact_toon }}` 注入機制：行級別 git diff + 拓撲地圖對照，
      促使 LLM 精準警告「這行修改會破壞 2 階以外某個遠端模組的呼叫鏈
      (Breaking Change Risk)」
- [ ] 工具面：`review_pull_request(workspace_key, git_diff, modified_files)`
      供 OpenCode / Cursor / LLM Agent 呼叫
- [ ] 改動面分析：需要動哪些檔案、最小侵入路徑（對照 `legacy/`）
- [ ] 對照結論：SDK 接入 vs 原生 rewrite 的取捨與適用情境

## 狀態

[待討論] — 依賴 graphify-sdk（Task 4）實際開發後才可撰寫實作細節；
目前保留定位、整合路徑與目錄結構。
