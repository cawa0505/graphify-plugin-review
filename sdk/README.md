# sdk/ — SDK 接入層草稿（Draft）

> 本目錄為「SDK 整合示範」角色（雙重定位之一）的草稿空間。
> **狀態：[待討論]** — Graphify SDK（Layer 2, 外部協議）本身尚未開發
> （GraphifyRust plugin-sdk-roadmap D5 語言順序暫緩），此處僅記錄接入層的
> 初步架構構想，作為日後 SDK 實作的語意參考。

## 定位

`legacy/code-review-graph/` 是「原本以 Python 寫的 MCP tool」。本目錄草稿回答：

- 未來 Graphify SDK（Layer 2, Stdio+JSON-RPC=MCP 協議）如何接入一個
  原本以不同語言撰寫的 tool？
- Python tool 需要最小的改動量是多少（adapter 模式 vs fork 重寫）？
- 接入後與原生 Rust rewrite（`crates/`）的工具面如何對齊？

## 草稿內容（規劃，尚未撰寫）

- [ ] SDK 協議對接面：tool 清單、`workspace_key` 綁定、事件訂閱（對齊
      `on_graph_updated` 語意）
- [ ] Python adapter 最小改動分析（對照 `legacy/code-review-graph/`）
- [ ] 雙模式對照：Mode 1 graphify-mcp gateway 彙整 vs Mode 2 獨立 MCP server

## 與原生 rewrite 的關係

- 兩條路徑共用同一組語意工具面：`review_impact` / `review_callers` /
  `review_entrypoints` / `review_flows`
- 效能評測（Task 4）以 `legacy/`（Python）為基準，對比 `crates/`（Rust 原生）
- SDK 接入層是第三種對照：Python tool 不重寫、透過 SDK 協議接入
