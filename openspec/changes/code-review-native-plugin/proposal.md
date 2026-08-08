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

We want a **first-party, native Rust review plugin** embedded in Graphify that:

1. Reuses Graphify Core's in-memory petgraph (`GraphOutput`) as the single source of truth — no duplicate
   graph, no SQLite mirror, no separate parser pipeline for the common path.
2. Exposes a **minimal, coherent set of `review*` tools** (detect_changes, impact radius, review context)
   registered automatically by GraphifyMCP — not 30 tools, not a standalone server.
3. Aligns with the plugin ecosystem contract: `workspace_key` routing (graphify-core v1), `sync_toon`
   payload exchange, and the same YAGNI/zero-mock rules as handoff/opendoc.

## Out of Scope (YAGNI)

- Porting all 30 upstream MCP tools / flows / communities / embeddings to Rust.
- Re-implementing a SQLite graph store — Graphify Core's petgraph is the graph.
- A standalone stdio/HTTP MCP server — graphify-mcp owns transport and registration.
- Web visualization (upstream D3.js) — not requested for the plugin.

## Proposed Direction

- **Crate**: `graphify-plugin-review` (package) / `graphify_plugin_review` (lib) — embedded plugin, no binary.
- **Trait**: implement `GraphifyPlugin` (get_id / bind / get_workspace_key / sync_toon / on_graph_updated).
- **Analysis on demand**: `detect_changes` (git diff → affected nodes via graph), `impact_radius`
  (BFS from changed nodes, default depth 2), `review_context` (token-efficient assembled context).
- **Zero mock**: analysis runs against the real in-memory graph and real git state; no fixtures, no fake data.
- **16ms Trace**: BFS impact trace budget is a **performance target** for the common path, not a hard SLA.

## Reference Material

- Upstream source (local-only, gitignored): `github-sourcecode/` — `tirth8205/code-review-graph` v2.3.6, MIT.
- Graphify Core v1 contract: `WorkspaceContext{workspace_key, workspace_name, root_path, timestamp}`,
  `GraphOutput` (petgraph), `sync_toon(Vec<u8>)`.
