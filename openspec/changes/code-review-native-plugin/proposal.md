# Proposal — Code Review Native Plugin

## Status

- **Date**: 2026-08-09
- **Decision**: Pending (documentation-first; crate skeleton awaits graphify-core v1 contract alignment)

## Problem Statement

Code review in AI coding tools currently relies on `code-review-graph` (Python, tirth8205) — a local-first
knowledge graph with 30+ MCP tools, SQLite persistence, and Tree-sitter parsing. This works, but it is a
**standalone process outside Graphify**: its graph is separate from Graphify Core's in-memory memory graph,
its tools are registered as an independent MCP server, and its runtime is Python (not native to the Graphify
Rust ecosystem).

This repo is a **dual-track evolutionary monorepo** — 雙軌平行 + 漸進式演進 — built around a
single Python starting point (`code-review-graph`), showing **two parallel evolution paths** and
the Graphify SDK's first-class client:

| 組件 | 落點 | 定位 |
|------|------|------|
| 1. **Rust 原生 Review Plugin** | `crates/graphify-plugin-review/` | 極致效能：16ms 零開銷、單一二進位、記憶體內直接操作 petgraph |
| 2. **Graphify Python SDK** (`graphify-sdk`) | `sdk/` | 官方對外 Python 打底基礎設施：高階 Async API、Stdio/JSON-RPC 封裝、workspace_key 透傳 |
| 3. **改版 Python Code Review MCP** | `python/review-mcp/` | SDK 的第一個 first-class client：全盤繼承 Python 資產 + `get_blast_radius` 降維打擊升級 |

Track A — native Rust rewrite (the subject of this change):

1. Reuses Graphify Core's in-memory petgraph (`GraphOutput`) as the single source of truth — no
   duplicate graph, no SQLite mirror, no separate parser pipeline for the common path.
2. Exposes a **minimal, coherent set of `review*` tools** (impact, callers, entrypoints, flows)
   registered automatically by GraphifyMCP — not 30 tools, not a standalone server.
3. Aligns with the plugin ecosystem contract: `workspace_key` routing (graphify-core v1), `sync_toon`
   payload exchange, and the same YAGNI/zero-mock rules as handoff/opendoc.

Track B — Graphify Python SDK + revamped MCP:

1. `GraphifyClient(workspace_key)` auto-handles Stdio/JSON-RPC with graphify-mcp and process lifecycle.
2. `get_blast_radius(git_diff/files, depth)` / `query_symbol_topology(symbol_name)` — high-level async API.
3. The revamped MCP keeps 100% of the mature Python skills/prompts/checks, and injects
   `{{ topology_impact_toon }}` into the review prompt so the LLM can flag cross-module
   breaking-change risks beyond line-level diff.

## Forward Contract (proposed, [待討論])

Track A proposes a **new trait method** on `GraphifyPlugin` (graphify-core v1 currently exposes only
`get_id / bind / get_workspace_key / sync_toon / on_graph_updated`):

```rust
fn analyze_diff_impact(&self, ctx: &WorkspaceContext, git_diff: &str, graph: &GraphOutput) -> ReviewOutput;
```

This is a **forward contract owned by GraphifyRust** (`graphify-core/src/plugin.rs`) — documented here
as proposed, not binding, until graphify-core lands it.

## SDK Language Order (supersedes D5)

SDK roadmap D5 originally ordered TS → Python → PHP → Rust → Go (暫緩). This change **promotes Python
first** (graphify-sdk as the official Python-facing infrastructure), superseding that order for Python.

## Out of Scope (YAGNI)

- Porting all 30 upstream MCP tools / flows / communities / embeddings to Rust.
- Re-implementing a SQLite graph store — Graphify Core's petgraph is the graph.
- A standalone stdio/HTTP MCP server — graphify-mcp owns transport and registration.
- Web visualization (upstream D3.js) — not requested for the plugin.
- Template engine (Tera/Jinja2) in Track A — `format!` suffices for MVP.

## Proposed Direction

- **Crate**: `graphify-plugin-review` (package) / `graphify_plugin_review` (lib) — embedded plugin, no binary.
- **Trait**: implement `GraphifyPlugin` + (proposed) `analyze_diff_impact`.
- **Analysis on demand**: `review_impact` (BFS from changed nodes, default depth 2), `review_callers`
  (reverse callers), `review_entrypoints` (no incoming `calls`), `review_flows` (path tracing).
- **Zero mock**: analysis runs against the real in-memory graph and real git state; no fixtures, no fake data.
- **16ms Trace**: BFS impact trace budget is a **performance target** for the common path, not a hard SLA.

## Reference Material

- Upstream source (fork, tracked): `legacy/code-review-graph/` — `tirth8205/code-review-graph` v2.3.6, MIT.
- Graphify Core v1 contract: `WorkspaceContext{workspace_key, workspace_name, root_path, timestamp}`,
  `GraphOutput` (petgraph), `sync_toon(Vec<u8>)`.
