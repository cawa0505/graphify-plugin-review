# graphify-plugin-review

Dual-role monorepo built around `code-review-graph` (Python, tirth8205) as the single
starting point, showing two evolution paths:

1. **Native Rust rewrite** — a first-party, embedded Rust crate (implementing the
   `GraphifyPlugin` trait) that provides structured code review analysis — impact
   radius, callers, entrypoints, flows — running directly on Graphify Core's
   in-memory petgraph. All `review*` tools are auto-registered by GraphifyMCP at
   startup.
2. **SDK integration demo** — how a future Graphify SDK (Layer 2) integrates an MCP
   tool originally written in a different language (Python): the fork in
   `legacy/code-review-graph/`, the SDK adapter draft in `sdk/`, and the
   walkthrough in `docs/integration/`.

> **Status**: documentation-first phase. Analysis targets and architecture are
> specified in `openspec/`; the crate skeleton awaits the graphify-core v1
> contract alignment.

## Repository layout

```
├── crates/                  # native Rust rewrite (embedded GraphifyPlugin)
├── sdk/                     # SDK adapter draft (Python tool ↔ SDK protocol)
├── legacy/code-review-graph/     # original Python tool (fork, reference)
├── docs/integration/        # SDK-into-foreign-MCP integration walkthrough
└── openspec/                # proposal / design / tasks (this change)
```

## Why native

The upstream `code-review-graph` (Python, [tirth8205](https://github.com/tirth8205/code-review-graph), MIT) proved
the review workflow, but as a standalone process it duplicates Graphify's graph,
persists its own SQLite store, and ships 30+ tools. The native plugin:

- **Reuses the in-memory graph** — analysis runs on Graphify's petgraph
  (`GraphOutput`) directly; no SQLite mirror, no duplicate parser pipeline.
- **Eliminates startup cost** — embedded crate, no interpreter boot or 30 MB
  dependency load.
- **Converges the tool set** — a minimal `review*` surface (impact, callers,
  entrypoints, flows), not 30 tools.
- **Zero mock** — every analysis runs against the real in-memory graph; change
  detection comes from Graphify Core's `on_graph_updated` (`modified_nodes`),
  not a re-implemented git diff.

### Performance targets (vs Python upstream)

| Operation | Python (ms) | Native target (ms) | Gain |
|-----------|------------|--------------------|------|
| impact BFS (1000 nodes) | 50–200 | 2–10 | 10–20x |
| flow trace | 20–100 | 2–5 | 10–20x |
| symbol search (10k nodes) | 100–500 | 5–20 | 5–20x |
| AST parse per file | 100–300 | 80–250 | 1.2–1.5x |

## Ecosystem alignment

- **Part of Graphify Plugins**: sibling to `graphify-plugin-handoff` and
  `graphify-plugin-opendoc`; plugins align on `workspace_key` (graphify-core v1
  contract) — no per-plugin walk-up.
- **Contract**: implements `GraphifyPlugin` (`get_id` / `bind` /
  `get_workspace_key` / `sync_toon`); transport and tool registration are owned
  by graphify-mcp.
- **Open-source safe**: no private hostnames, local IPs, or machine paths in
  version-controlled files.

## Development

- Build: `cargo build`
- Check/lint: `cargo check` / `cargo clippy`
- Test: `cargo test`

## Reference

- Upstream Python source is kept as a fork in `legacy/code-review-graph/`
  (reference material, `.git` stripped — tracked for the SDK integration demo).
- Full architecture and task breakdown: `openspec/changes/code-review-native-plugin/`.
