# Verification Core (`tools_v2/verification-core`)

## Phase-one implementation

The library is live with its existing flat source layout, including `src/proc.rs`. The decomposed process and test directories below are future targets. Public Rust imports use `verification_core` and the shared lock remains `target/.tbd-gate.lock`.

## Target architecture

Fail-closed, dependency-light repository verification and static analysis assertion library.

Formerly `crates/tbd-gate`. Developed to eliminate bash script defects in quality gates and enforce fail-closed invariants across CI, mod scripts, and platform contracts.

---

## 1. Core Primitives & Invariants

```text
tools_v2/verification-core/
├── Cargo.toml                           <-- Pinned dependencies: regex 1, libc 0.2
├── src/
│   ├── lib.rs                           <-- Core re-exports: Verdict, Report, Pattern, GateLock
│   ├── verdict.rs                       <-- 4-outcome non-collapsing Verdict enum (<350 LOC)
│   ├── gate.rs                          <-- Fail-closed file assertion primitives (<300 LOC)
│   ├── pattern.rs                       <-- Multi-line regex & literal search patterns (<200 LOC)
│   ├── scan.rs                          <-- Recursive fail-closed filesystem walker (<250 LOC)
│   ├── lock.rs                          <-- Unix flock concurrency lock (<300 LOC)
│   ├── report.rs                        <-- Standardized terminal reporter (<150 LOC)
│   └── proc/                            <-- Process execution with process-group isolation (<500 LOC)
│       ├── mod.rs                       <-- Interface re-exports
│       ├── runner.rs                    <-- setsid + killpg process group management (<400 LOC)
│       └── stream.rs                    <-- Concurrent thread-drained stdout/stderr (<300 LOC)
└── tests/
    └── proc_tests.rs                    <-- Extracted process execution unit tests
```

---

## 2. Invariant: The Four-Outcome Verdict

A standard `bool` or `Result` cannot carry the four distinct states required for reliable repository static analysis:

1. **`Held`**: The invariant condition was tested and held true.
2. **`Failed(Finding)`**: The invariant condition was tested and was violated.
3. **`DidNotRun(NotRun, Finding)`**: The check could not be completed because a prerequisite failed:
   - `TargetMissing`: The target source file does not exist.
   - `Unreadable`: The target file exists but could not be read (permissions/IO).
   - `ToolAbsent`: A required external binary is missing from `PATH` (e.g. exit 127).
   - `ToolError`: An external tool exited with an unhandled status.
   - `Signalled`: The process was killed by a signal (e.g. SIGKILL, SIGTERM).
   - `Timeout`: The check exceeded its allocated execution deadline.

> [!IMPORTANT]
> `Verdict` deliberately implements **NO `From<bool>`** and **NO `is_ok()`**. A caller cannot collapse "did not run" into "passed" by accident.

---

## 3. Exit Code Contract

Failures render with byte-for-byte fidelity matching CI scraping contracts:
- `0`: All assertions held.
- `1`: One or more assertions failed.
- `2`: One or more checks did not run (prioritized over exit 1).
