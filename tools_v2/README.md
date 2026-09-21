# Unified tooling

All four crates are live. xtask separates command domains, shared core helpers, and repository verifications. Developer-tools groups browser, Enfusion, raster, world export, and blueprint processing by responsibility.

- `verification-core`: fail-closed assertions, process isolation, and repository locking; no workspace dependencies.
- `ticket-engine`: typed storage and operations, validation, generated views, registry compatibility, ticket maintenance, wave scheduling/history, receipts, and estimates; no workspace dependencies.
- `developer-tools`: heavy CLI services, blueprint compilation, shared Enfusion archives, and engine-backed map verification. The executable names remain `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`.
- `xtask`: CLI dispatch, repository checks, and platform execution. Ticket and map adapters delegate to their domain crates. The map engine remains a transitive dependency through developer-tools, not a direct dependency. Its `deploy/`, `dedicated_server_profiles/` and `fixtures/mcp/` directories hold the data its commands read; `src/core/repository_layout.rs` names each location once.
- `enfusion_mcp_node_package`: the npm manifest, lockfile and node version that pin the `enfusion-mcp` server every agent session and MCP command starts. It sits outside every crate root so the dependency tree `npm ci` installs beside it is never walked by a crate-scoped file scan.

Ticketboard consumes ticket-engine’s existing public model and operation interfaces. Agent invocation, worktree cleanup, and platform-wave orchestration remain in xtask.

[ARCHITECTURE_PLAN.md](ARCHITECTURE_PLAN.md) states the invariants and the dependency direction;
[ANALYSIS_AND_INVENTORY.md](ANALYSIS_AND_INVENTORY.md) maps every module to what it is responsible
for. The five landing records carry the measurements taken as this tree was built:
[the relocation](PHASE_ONE_HANDOFF.md), [the heavy services](PHASE_TWO_HANDOFF.md),
[the ticket subsystem](PHASE_THREE_HANDOFF.md), [the decomposition](PHASE_FOUR_HANDOFF.md) and
[the closure](PHASE_FIVE_HANDOFF.md).
