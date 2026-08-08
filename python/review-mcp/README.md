# python/review-mcp/ — 改版 Python Review MCP

> **[軌 B]** graphify-sdk 的第一個「實戰練兵場 (First-class Client)」。
> **狀態：[待討論]** — 依賴 graphify-sdk（Task 4）實作後才開始。
> 整合過程與結論記錄於 `../docs/integration/`。

## 定位

直接拿原本成熟的 Python Code Review MCP（`../legacy/code-review-graph/`）引入
graphify-sdk，用最小成本把成熟的 Python Review Skills 升級成
「拓撲感知審查 (Topology-Aware Review)」。

## 核心需求（Task 5）

- **100% 繼承 Python 資產**：Skills、Prompt 範本、安全與效能檢查邏輯
- **導入 graphify-sdk 降維打擊**：收到 PR / git diff 請求時，透過
  `get_blast_radius(git_diff)` 取得 Graphify 16ms 算出的 `.toon` 拓撲地圖
- **拓撲增強型 Review Prompt**：行級別 git diff 注入
  `{{ topology_impact_toon }}` 變數，促使 LLM 精準警告「這行修改會破壞
  2 階以外某個遠端模組的呼叫鏈 (Breaking Change Risk)」
- **MCP 工具暴露**：`review_pull_request(workspace_key, git_diff, modified_files)`
  供 OpenCode / Cursor / LLM Agent 呼叫

## 規劃目錄

```
python/review-mcp/
├── pyproject.toml
└── review_mcp/
    ├── server.py      # MCP server 入口 + review_pull_request 工具
    ├── review.py      # 核心審查邏輯（繼承 legacy 資產）
    └── prompt.py      # {{ topology_impact_toon }} 注入
```
