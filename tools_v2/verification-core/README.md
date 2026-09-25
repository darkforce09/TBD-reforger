# Verification core

The `verification-core` crate, imported as `verification_core`: the small, fail-closed library that
every `cargo xtask` verification and many xtask commands are written with. It gives a check three
outcomes instead of two (held, failed, did not run), so a check whose input is missing, whose tool
is absent or whose child process was killed can never report a pass.

## Contents

```text
tools_v2/verification-core/
├── Cargo.toml  the `verification-core` library package; its only dependencies are `regex` and `libc`
└── src/        verdicts, pattern assertions, tree scans, child processes, the report and the lock
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

The two dependencies are there on purpose, and nothing else is: `regex` compiles the pattern
matcher into the gate, so no check depends on a search binary being installed, and `libc` provides
the two system calls the crate cannot do without, killing a timed-out child's whole process group
and `flock` on the shared verification lock. `src/README.md` describes each module.

The shared lock is `target/.repository-verification.lock` (`GATE_LOCK_RELPATH`), resolved against
the primary checkout found with `git rev-parse --git-common-dir`: a linked worktree that resolved
it against its own root would get a private lock that serialises nothing. `GateLock` has a private
field and no public constructor, so the only way to hold one is to have acquired it through
`flock_exclusive`, and running out of time yields `NotRun::Timeout`, a refusal, never an
unserialised run.

## Getting started

Run these from the repository root:

```bash
cargo test -p verification-core                 # the unit tests; they run sh, cat and sleep, and flock when present
cargo xtask verify readme-coverage --path tools_v2/verification-core   # any gate built on the crate
```

Neither the CI workflow nor the platform wave gate runs `cargo test -p verification-core`; run it
by hand after a change here.

## Configuration

The crate reads no environment variable itself, apart from `PATH` in `proc::which`. The lock
settings belong to its callers: `cargo xtask platform wave` takes the lock path from
`TBD_GATE_LOCK` (default: `GATE_LOCK_RELPATH` under the primary checkout), the heartbeat from
`TBD_GATE_LOCK_POLL` (30 s) and the deadline from `TBD_GATE_LOCK_MAX` (3600 s), in
`tools_v2/xtask/src/commands/platform/wave_execution/mod.rs`. The crate's own `DEFAULT_POLL` and
`DEFAULT_MAX` constants hold the same values.

## Public surface

- The library `verification_core`, with `Verdict`, `NotRun`, `Finding`, `Kind`, `Pattern`,
  `Report`, `GateLock` and `flock_exclusive` at its root and the modules `gate`, `scan`, `proc`,
  `report`, `lock`, `pattern` and `verdict`. The [source README](/tools_v2/verification-core/src/README.md)
  says which xtask folders use each.
- No binary.

## Boundaries

- Depends on: `regex` 1 and `libc` 0.2, and no workspace crate.
- Used by: `tools_v2/xtask/` only, by path dependency: its verifications under
  `tools_v2/xtask/src/verifications/`, its command groups under `tools_v2/xtask/src/commands/`
  (the lock holders are the platform wave driver and the MCP broker start in
  `tools_v2/xtask/src/commands/mcp/call.rs`), and `tools_v2/xtask/src/core/`.
- Rules:
  - the crate depends on no workspace crate (`foundational_engines_have_no_workspace_dependencies`
    in `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`);
  - production files stay under 500 lines and test files under 1,000, with tests in separate
    files (`tooling_source_files_stay_below_their_structural_limits` and
    `tooling_test_modules_live_in_separate_files`, same file);
  - every tracked file here is held to the prose rules of
    `tools_v2/xtask/src/tests/tooling_prose_rules.rs`;
  - "did not run" never becomes a pass, which each module's tests in `src/tests/` hold.

## Related documentation

- [Verifications](/tools_v2/xtask/src/verifications/README.md) — the gates built on this crate.
- [Documentation gates](/tools_v2/xtask/src/verifications/documentation/README.md) — the README
  and link gates, as one example of a gate family.
- [Tooling architecture](/documentation_v2/tools_v2/tooling_architecture.md) — the one outcome
  vocabulary and the shared lock across the tooling.
