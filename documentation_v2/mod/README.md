**Status:** live

# Docs index

| File | Purpose |
|---|---|
| [`SPAWN_DETERMINISM.md`](/documentation_v2/runbooks/spawn_determinism.md) | Spawn/equip determinism program (T-274) — xtask gate, asserts, verify-log pointer |
| [`STAGING-SERVER.md`](/documentation_v2/runbooks/game_server_staging/README.md) | Bootstrap + deploy to `192.168.0.140`, Direct Join troubleshooting, client setup |
| [`MCP_TOOLING.md`](/documentation_v2/runbooks/enfusion_mcp_tooling.md) | `cargo xtask mcp call` / warm daemon / exit codes / verification (shipped @ `e7e7232`) |
| [`discord-milestone-1-post.md`](/documentation_v2/archive/product_plans/discord_milestone_1_post.md) | Copy/paste Discord announcement for Milestone #1 (22 Aug 2026) |

**Scheduling:** [`MILESTONES.md`](/documentation_v2/archive/product_plans/mod_milestones.md)  
**Claude Code entry:** [`CLAUDE-CODE-START.md`](/documentation_v2/runbooks/mod_slice_workflow.md)  
**Full handoff:** [`CLAUDE-CONTINUATION.md`](/documentation_v2/archive/handoffs_and_kickoffs/mod_claude_continuation.md)

**Verification:** `cargo xtask mcp selftest` (offline MCP), `cargo xtask mcp smoke` (live Workbench), `cargo xtask mod spawn-determinism`, `cargo xtask mod spawn-verify`, `cargo xtask mod remote-logs` (staging), `cargo xtask debug direct-join` (LAN join)
