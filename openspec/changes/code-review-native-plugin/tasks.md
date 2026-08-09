# Tasks — graphify-plugin-review（B+A 混合模式）

> 對齊 design.md / proposal.md（2026-08-10 方向變更後）。

## Slice 0 — Pure Bridge & Core Binding（基礎單向鏈路，零網絡）

- [x] **T0.0 Repo 清理**：砍 `legacy/`（Python fork）、`sdk/`、`python/`、
      `docs/integration/`；雙語 README 重寫為 bridge 定位
- [x] **T0.1 Docs 重寫**：design / proposal / tasks（本文件）
- [x] **T0.2 Crate Setup & Trait Stub**：Cargo.toml + `ReviewPlugin` struct
      實作 `GraphifyPlugin` trait（get_id / bind / get_workspace_key /
      sync_toon / on_graph_updated）— commit c344dc4
- [x] **T0.3 Database Migration**：`registry.rs` — review_bindings DDL 建表
      + CRUD DAO（併入 graphify.db，`workspace_key` scoped PK）
      — commit c344dc4
- [x] **T0.4 File-based Ingest**：`ingest.rs` — IngestPayload JSON 解析
      + 轉譯（file → 待綁定 review 列表）— commit c344dc4
- [x] **T0.5 Line-to-Symbol Resolver**：`resolver.rs` — innermost span 匹配，
      `file_path + line_number` → canonical_node_id（`{file_path}:{kind}:{name}`
      原樣保留，含 extract 的 `./` 前綴以對齊 `modified_nodes`）— commit c344dc4
- [x] **T0.6 Graph Cache**：`sync.rs` — sync_toon 收圖 → from_toon（全寬容）→
      記憶體 GraphOutput 快取 — commit c344dc4
- [x] **T0.7 Domain Logic**：lib.rs — review_ingest / review_ingest_file /
      review_get_context / review_resolve 業務 API。**workspace_key 範圍規則**：
      bindings 以 plugin 當前 bound 的 `workspace_key` 為主（與 relay/opendoc
      一致）；`IngestPayload.workspace_key` 僅作 CRG 端 provenance 標記，不參與
      綁定查詢範圍 — commit 69fa8bb
- [x] **T0.8 CRG MCP Client Skeleton**：`crg_client.rs` — MCP Handshake +
      tools/call 骨架（ureq，Box 化 ureq::Error 避免 large_enum_variant），
      Slice 0 僅 framing，真呼叫 Slice 1/2 接 — commit c344dc4
- [x] **T0.9 Tests + graphify-cli/mcp 註冊驗證**：plugin 33/33 單元測試全綠、
      clippy clean；graphify-cli `review` 子指令 + graphify-mcp 3 個 review*
      工具 auto-register（reviewIngest / reviewGetContext / reviewResolve），
      MCP 21/21 測試通過；CLI + MCP e2e（fixture: extract `.toon` → ingest
      binding 2 + orphan 1 → get-context 命中 → resolve 翻狀態 → 再查為空）
      — plugin c344dc4 + 69fa8bb；GraphifyRust 424cd72

## Slice 1 — Drift Guard & Auto-Resolution（雙向銷案與漂移防禦）

> 細部 spec：design.md §7。CRG 端 RFC：`crg-requirements.md`。

- [ ] **T1.1 Signature Hash 範圍裁決**：實作 YAGNI 砍法（保留 schema
      欄但無比對路徑）；schema migration `ALTER TABLE review_bindings
      ADD COLUMN resolution_reason TEXT DEFAULT ''
      , ADD COLUMN resolved_at TEXT DEFAULT ''
      , ADD COLUMN resolved_by TEXT DEFAULT ''`
      — **[待使用者裁決 §7.2]**
- [ ] **T1.2 on_graph_updated Auto-Resolver**：對 workspace 內所有
      `status='unresolved' AND canonical_node_id != ''` 的 binding，檢查
      canonical_node_id 是否還存在於當前快取 GraphOutput 的節點集 — 不存在
      → 自動標 `resolved` + `resolved_by='auto:node_gone'` +
      `resolved_at=now()` + `resolution_reason` 留 plugin 預設值。
- [ ] **T1.3 review_resolve 工具完整化**：`review_resolve` /
      `reviewResolve` 接受新 `resolved_by` 與 `resolution_reason` 參數
      （手動 path）；本地 graphify.db 更新 + 回應中含完整狀態。CRG 端
      反向銷案走 `crg_client.resolve_review` — 此部分 **不阻塞 T1.2**，
      local 銷案在 CRG API 到位前可獨立 ship。
- [ ] **T1.4 CRG RFC 交付**：本文 `crg-requirements.md` 開出
      `search_reviews` / `resolve_review` 規格；提交給 CRG 端等待排程
      （graphify 端不依賴 R1/R2 已上線，純交付）。

## Slice 2 — Real-time Impact Guard（雙向主動衝擊防禦）

> 細部 spec：design.md §8。graphify-core v1.1 延伸需求見 §10。

- [ ] **T2.1 Impact Radius Inspection Engine**：在 `on_graph_updated`
      中以 `event.modified_nodes` 為種子，用 graphify-core `query_bfs`
      `max_depth=2` 逆向邊走 BFS。**前置確認**：
      graphify-core 有無公開 `GraphOutput → DiGraph` helper — 若無，
      提需求給 GraphifyRust 開 v1.1（或 plugin 端自寫轉換並標
      ponytail 註解）。
- [ ] **T2.2 ImpactAlert Domain Event 產出**：對 BFS 涵蓋集合內每個
      node，查 unresolved high/critical → 結構化 `ImpactAlert` 結構
      （design §8.2）。
- [ ] **T2.3 graphify-mcp 轉發 ImpactAlert**：與 GraphifyRust 協商
      trait v1.1 延伸（design §8.3 方案選定：推薦方案 A 透過 trait 注入
      notify callback closure）；溝通完成後
      graphify-mcp 端把 ImpactAlert 序列化為 MCP
      `notifications/review/impact_alert` 推送。
