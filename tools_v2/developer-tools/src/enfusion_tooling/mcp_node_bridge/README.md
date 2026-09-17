# MCP Node Bridge (`developer-tools/src/enfusion_tooling/mcp_node_bridge`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Pinned npm manifest for `enfusion-mcp: 0.6.1`.

Relocated from the legacy `scripts/mod/package.json` into this isolated submodule under `enfusion_tooling/`.

Provides the pinned tier-2 fallback for `xtask mcp call` when executing the external `enfusion-mcp` Node runner without an npx cold-start.
