# graphify-plugin-review

**Dual-Track Evolutionary monorepo** (雙軌平行 + 漸進式演進) built around
`code-review-graph` (Python, tirth8205) as the single starting point, running
two parallel evolution tracks:

**Track A — Native Rust rewrite** (`crates/`): a first-party, embedded Rust crate
(implementing the `GraphifyPlugin` trait) providing structured code review analysis —
impact radius, callers, entrypoints, flows — running directly on Graphify Core's
in-memory petgraph. `review*` tools are auto-registered by GraphifyMCP at startup.
Target: 16ms zero-overhead, single binary, memory-level git-diff topology parsing
(via the proposed `analyze_diff_impact` trait method).

**Track B — Graphify Python SDK + revamped MCP** (`sdk/` + `python/review-mcp/`):
`graphify-sdk` is Graphify's official Python-facing infrastructure (high-level async
API, Stdio/JSON-RPC encapsulation, `workspace_key` passthrough). The revamped Python
Review MCP is its **first first-class client**: it inherits 100% of the mature Python
review skills and upgrades them into topology-aware review by injecting
`{{ topology_impact_toon }}` from `get_blast_radius(git_diff)`.

> **Status**: documentation-first phase. Analysis targets and architecture are
> specified in `openspec/`; the crate skeleton awaits the graphify-core v1
> contract alignment (already verified against `graphify-core/src/`).

## Repository layout

```
├── crates/
│   └── graphify-plugin-review/   # [Track A] native Rust rewrite (embedded GraphifyPlugin)
├── sdk/                          # [Track B] graphify-sdk (official Python SDK, extractable structure)
├── python/
│   └── review-mcp/               # [Track B] revamped Python Review MCP (SDK first-class client)
├── legacy/code-review-graph/     # original Python tool (fork, reference)
├── docs/integration/             # SDK-into-foreign-MCP integration walkthrough
└── openspec/                     # proposal / design / tasks (this change)
```

## Why native (Track A)

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

## Why SDK + revamped MCP (Track B)

- `GraphifyClient(workspace_key)` auto-handles Stdio/JSON-RPC with graphify-mcp and
  process lifecycle — zero boilerplate for Python developers/agents.
- `get_blast_radius(git_diff, depth)` / `query_symbol_topology(symbol_name)` pull
  `.toon`-compressed topology in 16ms (core-computed).
- The revamped MCP keeps 100% of the mature Python assets (skills, prompt templates,
  safety/perf checks) and adds topology-aware review: the LLM can warn about
  "breaking change risk beyond the 2nd hop" instead of reviewing line-level diff alone.

## Ecosystem alignment

- **Part of Graphify Plugins**: sibling to `graphify-plugin-handoff` and
  `graphify-plugin-opendoc`; plugins align on `workspace_key` (graphify-core v1
  contract) — no per-plugin walk-up.
- **Contract**: implements `GraphifyPlugin` (`get_id` / `bind` /
  `get_workspace_key` / `sync_toon` / `on_graph_updated`); transport and tool
  registration are owned by graphify-mcp. `analyze_diff_impact` is a proposed
  forward contract owned by graphify-core (see `openspec/`).
- **Open-source safe**: no private hostnames, local IPs, or machine paths in
  version-controlled files.

## Development

- Track A (Rust): `cargo build` / `cargo check` / `cargo clippy` / `cargo test`
- Track B (Python): see `sdk/README.md` and `docs/integration/`

## Reference

- Upstream Python source is kept as a fork in `legacy/code-review-graph/`
  (reference material, `.git` stripped — tracked for the SDK integration demo).
- Full architecture and task breakdown: `openspec/changes/code-review-native-plugin/`.
