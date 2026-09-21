# Verification Core (`tools_v2/verification-core`)

Fail-closed, dependency-light repository verification and static-analysis assertion library. Every
quality gate in `xtask` and the heavy tooling in `developer-tools` links it, so it stays small on
purpose: `regex` (so pattern matching needs no external search binary) and `libc` (so a timed-out
process group can be killed and the shared verification lock can be `flock`ed). Rust callers
import it as `verification_core`.

---

## 1. Layout

```text
tools_v2/verification-core/
├── Cargo.toml                    <-- Pinned dependencies: regex 1, libc 0.2
└── src/
    ├── lib.rs                    <-- Re-exports: Verdict, Finding, Kind, NotRun, Pattern, Report, GateLock
    ├── verdict.rs                <-- The four-outcome Verdict enum and its rendering
    ├── gate.rs                   <-- Fail-closed file and text assertion primitives
    ├── pattern.rs                <-- Multi-line regex and literal search patterns
    ├── scan.rs                   <-- Recursive fail-closed filesystem walker
    ├── lock.rs                   <-- Unix flock serialisation over the shared lock file
    ├── report.rs                 <-- Terminal reporter and the process exit contract
    ├── proc/
    │   ├── mod.rs                <-- Run, Output and Merged: what a child process is and produced
    │   ├── runner.rs             <-- Spawn, process-group isolation, deadline, group kill
    │   ├── stream.rs             <-- Concurrent pipe drains, so a full buffer cannot deadlock
    │   └── lookup.rs             <-- PATH resolution, retry with backoff, wait-for-condition
    └── tests/                    <-- One test file per module, declared with #[path] from it
        ├── gate_tests.rs
        ├── lock_tests.rs
        ├── pattern_tests.rs
        ├── proc_tests.rs
        ├── report_tests.rs
        ├── scan_tests.rs
        └── verdict_tests.rs
```

---

## 2. Invariant: the four-outcome verdict

A `bool` or a `Result` cannot carry the four states repository static analysis needs:

1. **`Held`** — the invariant was tested and held.
2. **`Failed(Finding)`** — the invariant was tested and was violated.
3. **`DidNotRun(NotRun, Finding)`** — a prerequisite failed, so nothing was tested:
   - `TargetMissing`: the target source file does not exist.
   - `Unreadable`: the target exists but could not be read (permissions or IO).
   - `ToolAbsent`: a required external binary is missing from `PATH`.
   - `ToolError`: an external tool exited with an unhandled status.
   - `Signalled`: the process was killed by a signal.
   - `Timeout`: the check exceeded its execution deadline.

> [!IMPORTANT]
> `Verdict` implements **no `From<bool>`** and **no `is_ok()`**. A caller cannot collapse "did not
> run" into "passed" by accident, and adding a `NotRun` variant breaks every incomplete `match` in
> the workspace rather than silently widening one.

---

## 3. Exit code contract

`Report::finish` renders the summary and returns the process exit code:

- `0`: every check ran and held.
- `1`: one or more checks failed and every check ran.
- `2`: one or more checks did not run — prioritised over exit 1, because a check that did not run
  is not a pass.

---

## 4. The shared verification lock

`lock::GATE_LOCK_RELPATH` is `target/.repository-verification.lock`, and `TBD_GATE_LOCK` overrides
it. It is repository-relative and must be resolved against the **primary** repository root
(`git rev-parse --git-common-dir`): a linked worktree that resolves it against its own root gets a
private lock file that serialises nothing. `cargo xtask platform wave` is the holder, and its
`TBD_GATE_LOCK_POLL` and `TBD_GATE_LOCK_MAX` overrides set the heartbeat interval and the refusal
deadline (30 seconds and 1 hour by default).

`GateLock` has a private field and no public constructor, so the only way to hold one is to have
acquired it through `flock_exclusive`, and exhausting the deadline yields `NotRun::Timeout` — a
refusal, never an unserialised run.
