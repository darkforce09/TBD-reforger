# Unified tooling

All four crates are live. xtask separates command domains, shared core helpers, and repository verifications. Developer-tools groups browser, Enfusion, raster, world export, and blueprint processing by responsibility.

- `verification-core`: fail-closed assertions, process isolation, and repository locking; no workspace dependencies.
- `ticket-engine`: typed storage and operations, validation, generated views, registry compatibility, ticket maintenance, wave scheduling/history, receipts, and estimates; no workspace dependencies.
- `developer-tools`: heavy CLI services, blueprint compilation, shared Enfusion archives, and engine-backed map verification. The executable names remain `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`.
- `xtask`: CLI dispatch, repository checks, and platform execution. Ticket and map adapters delegate to their domain crates. The map engine remains a transitive dependency through developer-tools, not a direct dependency. Its `deploy/`, `dedicated_server_profiles/` and `fixtures/mcp/` directories hold the data its commands read; `src/core/repository_layout.rs` names each location once.
- `enfusion_mcp_node_package`: the npm manifest, lockfile and node version that pin the `enfusion-mcp` server every agent session and MCP command starts. It sits outside every crate root so the dependency tree `npm ci` installs beside it is never walked by a crate-scoped file scan.

Ticketboard consumes ticket-engine’s existing public model and operation interfaces. Agent invocation, worktree cleanup, and platform-wave orchestration remain in xtask.

The `tools_v2` refactor program's records are archived and frozen; the code is the live authority.
[The architecture plan](/documentation_v2/archive/tools_v2_refactor/architecture_plan.md) is the
program's statement of the invariants and the dependency direction;
[the analysis and inventory](/documentation_v2/archive/tools_v2_refactor/analysis_and_inventory.md)
is its map of every module to what it is responsible for. The five landing records carry the
program's measurements:
[the relocation](/documentation_v2/archive/tools_v2_refactor/phase_one_handoff.md),
[the heavy services](/documentation_v2/archive/tools_v2_refactor/phase_two_handoff.md),
[the ticket subsystem](/documentation_v2/archive/tools_v2_refactor/phase_three_handoff.md),
[the decomposition](/documentation_v2/archive/tools_v2_refactor/phase_four_handoff.md) and
[the closure](/documentation_v2/archive/tools_v2_refactor/phase_five_handoff.md).
