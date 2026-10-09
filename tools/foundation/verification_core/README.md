# Verification core

The `verification_core` crate: the small, fail-closed library every `cargo xtask` verification and
many xtask commands are written with. It gives a check three outcomes instead of two (held,
failed, did not run), so a check whose input is missing, whose tool is absent or whose child
process was killed can never report a pass. `process_runner` and `repository_laws` report in its
vocabulary.

## Contents

```text
tools/foundation/verification_core/
├── Cargo.toml  the `verification_core` library package: `regex` and `thiserror`, layout tier 0
└── src/        verdicts, pattern assertions, tree scans, the report, the lock, the error and the prelude
```

## How it works

A check produces a `Verdict`: `Held`, `Failed` with a rendered finding, or `DidNotRun` with the
`NotRun` cause (a missing or unreadable target, an absent or failing tool, a signal, a timeout).
`Verdict` has no `From<bool>` and no `is_ok()`, so no expression folds "did not run" into "passed"
by accident, and adding a `NotRun` variant breaks every `match` that does not handle it. A gate
passes its verdicts to a `Report`, whose `finish` returns the process exit code:

| Exit | Meaning |
|---|---|
| 0 | every check ran and held |
| 1 | at least one check failed, and every check ran |
| 2 | at least one check did not run; this outranks 1, since a check that did not run is not a pass |

`regex` compiles the pattern matcher into the gate, so no check depends on a search binary being
installed. The lock takes a non-blocking exclusive `flock` through `File::try_lock`, the same
primitive `flock(1)` takes. `src/README.md` describes each module.

The shared lock is `target/.repository-verification.lock` (`GATE_LOCK_RELPATH`), resolved against
the primary checkout found with `git rev-parse --git-common-dir`: a linked worktree that resolved
it against its own root would get a private lock that serialises nothing. `GateLock` has a private
field and no public constructor, so the only way to hold one is to have acquired it through
`flock_exclusive`, and running out of time yields `NotRun::Timeout`, a refusal, never an
unserialised run.

## Getting started

Run these from the repository root:

```bash
cargo test -p verification_core   # the unit tests; they run sh, cat and sleep, and flock when present
cargo xtask verify readme-coverage --path tools/foundation/verification_core   # any gate built on the crate
```

## Configuration

No feature, and the crate reads no environment variable. The lock settings belong to its callers:
`cargo xtask platform wave` takes the lock path from `TBD_GATE_LOCK` (default:
`GATE_LOCK_RELPATH` under the primary checkout), the heartbeat from `TBD_GATE_LOCK_POLL` (30 s)
and the deadline from `TBD_GATE_LOCK_MAX` (3600 s), in
`tools/commands/platform_execution/src/wave_execution/mod.rs`. The crate's own `DEFAULT_POLL` and
`DEFAULT_MAX` constants hold the same values.

## Public surface

- At the crate root: `Verdict`, `NotRun`, `Finding`, `Kind`, `Pattern`, `Report`, `GateLock`,
  `flock_exclusive`, `Error` and `Result`; the modules `gate`, `scan`, `report`, `lock`,
  `pattern`, `verdict` and `prelude`. The
  [source README](/tools/foundation/verification_core/src/README.md) says which folders use each.
- No binary.

## Boundaries

- Depends on: `regex` and `thiserror`, and no workspace crate.
- Used by: `process_runner` and `repository_laws`, which report through `NotRun` and `Verdict`;
  the check crates under `tools/checks/` and the command crates under `tools/commands/`;
  `xtask`: its command groups under
  `tools/xtask/src/commands/` (the lock holders are the platform wave driver and the MCP broker
  start in `tools/commands/enfusion_mcp/src/call.rs`); and `api`, as a
  dev-dependency, whose `crates/api/api_server/tests/engineering_laws.rs` uses its patterns and scans.
- Rules:
  - the crate depends on no workspace crate
    (`foundation_crates_depend_only_on_lower_foundation_crates` in
    `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`; tier 0 in
    `cargo xtask verify crate-tiers`);
  - production files stay under 500 lines and test files under 1,000, with tests in separate
    files (`tooling_source_files_stay_below_their_structural_limits` and
    `tooling_test_modules_live_in_separate_files`, same file);
  - every tracked file here is held to the prose rules of
    `tools/checks/repository_checks/src/tests/tooling_prose_rules.rs`;
  - "did not run" never becomes a pass, which each module's tests in `src/tests/` hold.

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the three crates and their tiers.
- [Check crates](/tools/checks/README.md) — the gates built on this crate.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — the one outcome
  vocabulary and the shared lock across the tooling.
