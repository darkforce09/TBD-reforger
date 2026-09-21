# Unified tooling

All four crates are live. xtask separates command domains, shared core helpers, and repository verifications. Developer-tools groups browser, Enfusion, raster, world export, and blueprint processing by responsibility.

- `verification-core`: fail-closed assertions, process isolation, and repository locking; no workspace dependencies.
- `ticket-engine`: typed storage and operations, validation, generated views, registry compatibility, ticket maintenance, wave scheduling/history, receipts, and estimates; no workspace dependencies.
- `developer-tools`: heavy CLI services, blueprint compilation, shared Enfusion archives, and engine-backed map verification. The executable names remain `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`.
- `xtask`: CLI dispatch, repository checks, and platform execution. Ticket and map adapters delegate to their domain crates. The map engine remains a transitive dependency through developer-tools, not a direct dependency.

Ticketboard consumes ticket-engine’s existing public model and operation interfaces. Agent invocation, worktree cleanup, and platform-wave orchestration remain in xtask.

See [the architecture plan](ARCHITECTURE_PLAN.md), [phase-two evidence](PHASE_TWO_HANDOFF.md), [phase-three evidence](PHASE_THREE_HANDOFF.md), and [phase-four status and evidence](PHASE_FOUR_HANDOFF.md).
