# Verification core source

The `verification_core` library: one vocabulary for what a repository check can conclude, and the
primitives every xtask gate is written with (pattern assertions over files, tree scans, child
processes, the run report and the shared verification lock), each built so that a check that
could not look never reads as a pass.

## Contents

```text
tools_v2/verification-core/src/
├── gate.rs     pattern assertions: `ban`, `require`, their `_str` forms and the `probe_*` pair
├── lib.rs      the crate root: the module list and the re-exports
├── lock.rs     `flock_exclusive` and `GateLock`: the exclusive lock that serialises expensive steps
├── pattern.rs  `Pattern`: a compiled regex or escaped literal whose `^` and `$` anchor lines
├── proc/       child processes with process-group isolation, deadlines and signal reporting
├── report.rs   `Report`: accumulates verdicts, prints failures as they land, and yields the exit code
├── scan.rs     `walk_files`, `matching_lines` and `Hit`: a sorted tree walk and the lines that match
├── tests/      unit tests for every module, one file each
└── verdict.rs  `Verdict`, `NotRun`, `Finding` and `Kind`: the three outcomes and their rendering
```

## How it works

```text
gate::ban / require          ─┐
proc::Run::expect_ok / code  ─┼──► Verdict ──► Report::check ──► Report::finish ──► exit 0, 1 or 2
your own match on NotRun     ─┘
scan::walk_files + matching_lines ──► Result<Vec<Hit>, NotRun>   (the caller renders path:line:text)
```

- A check ends in one `Verdict`: `Held`, `Failed(Finding)`, or `DidNotRun(NotRun, Finding)`.
  `NotRun` names why nothing was examined: `TargetMissing`, `Unreadable`, `ToolAbsent`,
  `ToolError`, `Signalled` or `Timeout`. `Verdict` has no conversion to or from `bool`, so a
  caller must `match` it, and a new `NotRun` variant breaks every incomplete match.
- `gate.rs` reads every target before matching and answers `DidNotRun` for a missing or
  unreadable file; `probe_files` returns `Result<bool, NotRun>` for conditions built from several
  matches, so `?` carries "did not run" upward.
- `Pattern` compiles with the `regex` crate in multi-line mode, so `^` and `$` mean line start and
  end, as in a line-oriented search tool; no external search binary is ever run.
- `scan::walk_files` fails on a missing root or an unreadable folder, never follows symbolic links,
  and returns paths sorted.
- `Report::finish` prints `<label>: OK — N check(s), all held` and returns 0, or a failure summary
  and returns 1 when every check ran, 2 when any did not; `Verdict::into_exit` maps one verdict the
  same way, and `into_binary_exit_code` folds "did not run" into 1 for the gates whose callers
  read only pass or fail.
- A failure renders as `FAIL: <headline>` and continuation lines indented by six spaces, the same
  in every gate, so one log reads like another.
- `lock.rs`: `GATE_LOCK_RELPATH` is `target/.repository-verification.lock`, relative to the
  primary checkout (`git rev-parse --git-common-dir`), so every linked worktree shares it.
  `flock_exclusive` polls a non-blocking `flock` every 50 ms, calls the heartbeat every `poll`,
  and returns `NotRun::Timeout` at `max` or `NotRun::ToolError` on any error other than "held by
  another process". A `GateLock` can only come from a successful acquisition and releases on drop.
  `DEFAULT_POLL` (30 s) and `DEFAULT_MAX` (1 h) are the documented defaults; the caller reads any
  override.

## Public surface

- At the crate root: `Verdict`, `NotRun`, `Finding`, `Kind`, `Pattern`, `Report`, `GateLock` and
  `flock_exclusive`, imported across `tools_v2/xtask/src/verifications/` and
  `tools_v2/xtask/src/commands/`.
- `gate`: the verifications and the `debug`, `deploy` and `mcp` command groups.
- `scan`: the `architecture`, `database`, `language_bans`, `licensing` and `mod_scripts`
  verifications, the CI task runner and `mod_ops/mission_test.rs`.
- `proc`: every xtask module that runs an external program; the `proc/` README lists them.
- `lock`: the platform wave driver (`tools_v2/xtask/src/commands/platform/wave_execution/lock.rs`
  and `mod.rs`, which resolves `GATE_LOCK_RELPATH`) and the MCP broker start
  (`tools_v2/xtask/src/commands/mcp/call.rs`, one lock per socket).

## Boundaries

- Depends on: `std`, `regex` and `libc` only.
- Used by: `tools_v2/xtask/`, the only crate that depends on it.
- Rules:
  - "did not run" never folds into a pass: a missing target is `DidNotRun`
    (`missing_target_is_did_not_run_not_held` in `tests/gate_tests.rs`), a missing scan root is
    never zero hits (`a_missing_root_is_did_not_run_not_zero_hits`), a lock that times out is
    `DidNotRun` (`exhaustion_is_did_not_run_never_a_pass`), and a report with any check that did not
    run exits 2 (`a_did_not_run_outranks_violations`);
  - `^` and `$` stay line anchors (`caret_is_a_line_anchor`), and a match never spans two files
    (`patterns_cannot_match_across_a_file_boundary`);
  - the failure text keeps its headline and six-space continuation shape
    (`a_missing_target_names_the_file_and_the_six_space_continuation`).
