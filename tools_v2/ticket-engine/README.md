# Ticket engine

Self-contained ticket storage, validation, command services, sync outputs (queue JSON, roadmap markers, gap-analysis ticket columns), scheduling, and accounting. This crate has no internal workspace dependencies.

## Ownership

- `model`, `encoding`, `vocab`, and `timestamp`: public ticket types, canonical TOML, scope resolution, and UTC timestamps. Existing crate-root exports remain stable.
- `store` and `ops`: fail-closed corpus loading, validated post-images, and surgical atomic file replacement.
- `registry`: ticket files on disk, read-only value projections, and ticket statuses at a past revision. The whole-tree writer refuses a typed tree, so the typed operations are the only writer.
- `validation`: schema, vocabulary, ownership, body, readiness, shipping, hierarchy, accounting, debt, and repository-reference checks.
- `cli`: briefs, queries, mutations, shipping, batch selection, and configuration.
- `sync`: queue JSON, roadmap markers, and gap-analysis ticket columns.
- `wave_lock`: dependency packing, collision selection, deterministic lock files, drift checks, reservations, and decoding of the archived wave plans at the revisions that still carry them.
- `metrics`: measured receipts, elapsed time, token accounting, and derived estimates.
- `repository`: every repository path the crate reads or writes, spelled once, with a `documentation` submodule for the ones under the documentation tree; plus checkout-root discovery. `xtask` and `ticketboard` resolve those paths from here.

## Host interfaces

Filesystem services receive the active repository root. `cli::cmd_run` receives an execution callback; `cli::cleanup_targets` returns a worktree path and branch without deleting either. The host owns process invocation and cleanup. Shared `wave_lock::history` functions give the compiler and platform checks one close-marker authority while retaining their different HEAD and numbering policies.

Unit tests live in sibling `tests/` files. Production files remain below 500 lines, tests below 1,000. Execution receipt fixtures are shared with xtask and live in `tests/fixtures/execution_receipts`.

See the archived, frozen [phase-three verification](/documentation_v2/archive/tools_v2_refactor/phase_three_handoff.md) record of the `tools_v2` refactor program.
