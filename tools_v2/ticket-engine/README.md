# Ticket engine

Self-contained ticket storage, validation, command services, generated views, scheduling, and accounting. This crate has no internal workspace dependencies.

## Ownership

- `model`, `encoding`, `vocab`, and `timestamp`: public ticket types, canonical TOML, scope resolution, and UTC timestamps. Existing crate-root exports remain stable.
- `store` and `ops`: fail-closed corpus loading, validated post-images, and surgical atomic file replacement.
- `registry`: read-only projections, historical status lookup, and compatibility formats. Legacy writers refuse the live typed format.
- `validation`: schema, vocabulary, ownership, body, readiness, shipping, hierarchy, accounting, debt, and repository-reference checks.
- `cli`: briefs, queries, mutations, shipping, batch selection, and configuration.
- `sync`: six Markdown views, queue JSON, roadmap markers, and gap-analysis ticket columns.
- `wave_lock`: dependency packing, collision selection, deterministic lock files, drift checks, reservations, and historical plan decoding.
- `metrics`: measured receipts, elapsed time, token accounting, and derived estimates.
- `maintenance`: ticket migrations, timestamp provenance, and body quarantine.
- `repository`: explicit ticket paths and active-checkout root discovery.

## Host interfaces

Filesystem services receive the active repository root. `cli::cmd_run` receives an execution callback; `cli::cleanup_targets` returns a worktree path and branch without deleting either. The host owns process invocation and cleanup. Shared `wave_lock::history` functions give the compiler and platform checks one close-marker authority while retaining their different HEAD and numbering policies.

Unit tests live in sibling `tests/` files. Production files remain below 500 lines, tests below 1,000. Execution receipt fixtures are shared with xtask and live in `tests/fixtures/execution_receipts`.

See [phase-three verification](../PHASE_THREE_HANDOFF.md).
