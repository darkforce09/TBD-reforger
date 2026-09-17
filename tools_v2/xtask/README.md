# Workspace task runner

`cargo xtask` preserves the existing CLI routes and exit behavior.

- `commands/ticket` forwards ticket services to ticket-engine and implements platform-side agent execution and cleanup.
- `commands/wave` forwards lock compilation/checks and ticket collision analysis.
- `commands/map` and `verifications/map_assets` delegate heavy processing to developer-tools.
- `wave` retains platform-wave orchestration and consumes ticket-engine’s shared history rules.
- `slice_run` retains agent process execution and writes receipts through ticket-engine.

Receipt fixtures live in `../ticket-engine/tests/fixtures/execution_receipts`; blueprint fixtures live in `../developer-tools/test_fixtures/blueprint`. Tests resolve fixtures from the active repository.

The CLI declarations and unrelated repository checks retain their current layout. The general router rewrite and verification decomposition belong to phase four in [the architecture plan](../ARCHITECTURE_PLAN.md). See [the phase-three handoff](../PHASE_THREE_HANDOFF.md) for checks and known baseline failures.
