# Docs index

| File | Purpose |
|---|---|
| [`SPAWN_DETERMINISM.md`](SPAWN_DETERMINISM.md) | Spawn/equip determinism program (T-274) — xtask gate, asserts, verify-log pointer |
| [`STAGING-SERVER.md`](STAGING-SERVER.md) | Bootstrap + deploy to `192.168.0.140`, Direct Join troubleshooting, client setup |
| [`MCP_TOOLING.md`](MCP_TOOLING.md) | `cargo xtask mcp call` / warm daemon / exit codes / verification (shipped @ `e7e7232`) |
| [`discord-milestone-1-post.md`](discord-milestone-1-post.md) | Copy/paste Discord announcement for Milestone #1 (22 Aug 2026) |

**Scheduling:** [`MILESTONES.md`](MILESTONES.md)  
**Claude Code entry:** [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md)  
**Full handoff:** [`CLAUDE-CONTINUATION.md`](CLAUDE-CONTINUATION.md)

**Verification:** `cargo xtask mcp selftest` (offline MCP), `cargo xtask mcp smoke` (live Workbench), `cargo xtask mod spawn-determinism`, `cargo xtask mod spawn-verify`, `cargo xtask mod remote-logs` (staging), `cargo xtask debug direct-join` (LAN join)
