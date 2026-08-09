# Tasks — graphify-plugin-review（B+A 混合模式）

> 對齊 design.md / proposal.md（2026-08-10 方向變更後）。

## Slice 0 — Pure Bridge & Core Binding（基礎單向鏈路，零網絡）

- [x] **T0.0 Repo 清理**：砍 `legacy/`（Python fork）、`sdk/`、`python/`、
      `docs/integration/`；雙語 README 重寫為 bridge 定位
- [x] **T0.1 Docs 重寫**：design / proposal / tasks（本文件）
- [ ] **T0.2 Crate Setup & Trait Stub**：Cargo.toml + `ReviewPlugin` struct
      實作 `GraphifyPlugin` trait（get_id / bind / get_workspace_key /
      sync_toon / on_graph_updated）
- [ ] **T0.3 Database Migration**：`registry.rs` — review_bindings DDL 建表
      + CRUD DAO（併入 graphify.db）
- [ ] **T0.4 File-based Ingest**：`ingest.rs` — IngestPayload JSON 解析
      + 轉譯（file → 待綁定 review 列表）
- [ ] **T0.5 Line-to-Symbol Resolver**：`resolver.rs` — 對 GraphOutput 線掃，
      `file_path + line_number` → canonical_node_id
- [ ] **T0.6 Graph Cache**：`sync.rs` — sync_toon 收圖 → from_toon →
      記憶體 GraphOutput 快取
- [ ] **T0.7 Domain Logic**：`review.rs` — review_ingest / review_get_context
      / review_resolve 業務 API
- [ ] **T0.8 CRG MCP Client Skeleton**：`crg_client.rs` — MCP Handshake
      + tools/call 骨架（對接 CRG 4 tools），Slice 0 僅骨架不阻塞
- [ ] **T0.9 Tests + graphify-mcp 註冊驗證**：單元測試全綠；
      graphify-mcp auto-registration e2e

## Slice 1 — Drift Guard & Auto-Resolution（雙向銷案與漂移防禦）

- [ ] **T1.1 Signature Hash Tracking**：寫入 review_bindings 時計算
      AST 節點結構 hash
- [ ] **T1.2 on_graph_updated Auto-Resolver**：偵測 AST 變動，若問題已
      修正則更新 status = resolved
- [ ] **T1.3 review_resolve 工具完整化**：手動/自動銷案介面
- [ ] **T1.4 CRG RFC 開出**：`search_reviews` / `resolve_review` 需求規格
      （選項 C）

## Slice 2 — Real-time Impact Guard（雙向主動衝擊防禦）

- [ ] **T2.1 Impact Radius Inspection**：on_graph_updated 沿變動節點 BFS
      衝擊半徑，檢查觸及未解決 high/critical Review 節點
- [ ] **T2.2 ImpactAlert domain event**：產出 event 交 graphify-mcp 轉發
      `notifications/review/impact_alert`
- [ ] **T2.3 graphify-mcp 協商**：與 GraphifyRust 協商 ImpactAlert 轉發機制
