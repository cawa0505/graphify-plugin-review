# docs/integration/ — SDK 整合紀錄

> 本目錄紀錄「怎麼把 SDK library 整合進原本是不同語言寫的 MCP 裡」的過程
> 與結論 — 以 `legacy/code-review-graph/`（Python）為具體案例。

## 內容（規劃）

- [ ] 整合 walkthrough：Python tool → SDK 協議 → GraphifyMCP 註冊
- [ ] 改動面分析：需要動哪些檔案、最小侵入路徑
- [ ] 對照結論：SDK 接入 vs 原生 rewrite 的取捨與適用情境

## 狀態

[待討論] — 依賴 Graphify SDK（Layer 2）實際開發後才可撰寫實作細節；
目前僅保留定位與目錄結構。
