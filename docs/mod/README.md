# Docs index

| File | Purpose |
|---|---|
| [`SPAWN_DETERMINISM.md`](SPAWN_DETERMINISM.md) | Spawn/equip determinism program (T-274) — Makefile targets, asserts, verify-log pointer |
| [`STAGING-SERVER.md`](STAGING-SERVER.md) | Bootstrap + deploy to `192.168.0.140`, Direct Join troubleshooting, client setup |
| [`MCP_TOOLING.md`](MCP_TOOLING.md) | `mcp-call.sh` / warm daemon / exit codes / verification (shipped @ `e7e7232`) |
| [`discord-milestone-1-post.md`](discord-milestone-1-post.md) | Copy/paste Discord announcement for Milestone #1 (22 Aug 2026) |

**Scheduling:** [`MILESTONES.md`](MILESTONES.md)  
**Claude Code entry:** [`CLAUDE-CODE-START.md`](CLAUDE-CODE-START.md)  
**Full handoff:** [`CLAUDE-CONTINUATION.md`](CLAUDE-CONTINUATION.md)

**Verification scripts:** `scripts/mod/mcp-call-selftest.sh` (offline MCP), `cargo xtask mcp smoke` (live Workbench), `cargo xtask mod spawn-determinism` (`make mod-spawn-determinism`), `cargo xtask mod spawn-verify`, `scripts/mod/remote-log-grep.sh` (staging), `cargo xtask debug direct-join` (LAN join)
