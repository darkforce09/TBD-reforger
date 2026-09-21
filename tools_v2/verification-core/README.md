# Verification Core (`tools_v2/verification-core`)

Fail-closed, dependency-light repository verification and static-analysis assertion library. Every
quality gate in `xtask` and the heavy tooling in `developer-tools` links it, so it stays small on
purpose: `regex` (so pattern matching needs no external search binary) and `libc` (so a timed-out
process group can be killed and the shared gate lock can be `flock`ed). Rust callers import it as
`verification_core`.

---

## 1. Layout

```text
tools_v2/verification-core/
├── Cargo.toml                <-- Pinned dependencies: regex 1, libc 0.2
└── src/
    ├── lib.rs                <-- Re-exports: Verdict, Finding, Kind, NotRun, Pattern, Report, GateLock
    ├── verdict.rs            <-- The four-outcome Verdict enum and its rendering
    ├── gate.rs               <-- Fail-closed file and text assertion primitives
    ├── pattern.rs            <-- Multi-line regex and literal search patterns
    ├── scan.rs               <-- Recursive fail-closed filesystem walker
    ├── lock.rs               <-- Unix flock concurrency lock over the shared gate lock file
    ├── report.rs             <-- Terminal reporter and the process exit contract
    └── proc.rs               <-- Child process execution with process-group isolation
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
