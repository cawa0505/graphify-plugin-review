# sdk/

**這個資料夾已獨立為官方 repo：<https://github.com/cawa0505/graphify-sdk-python>**

Graphify Python SDK（Layer 2 外部 SDK）不再在此 monorepo 內開發。本目錄僅保留
指向說明：

- **Repo**：`cawa0505/graphify-sdk-python`
- **整合故事**：改版 Python Review MCP（`../python/review-mcp/`）以
  `pip install graphify-sdk-python` 方式依賴此 SDK — 整合過程與結論記錄於
  `../docs/integration/`。
- **生態定位**：SDK 家族 per-language repo（Python 優先），協議 spec 集中在
  GraphifyRust；各語言 SDK 不互相內嵌，依賴走套件管理（PyPI / npm / …）。
