# Verification Core (`tools_v2/verification-core/`)

A dependency-light, fail-closed static assertion library (`tools_v2/verification-core`).

## Key Invariants
- `Verdict`: 4-outcome non-collapsing verification enum (`Held`, `Failed`, `DidNotRun`). No `From<bool>` or `is_ok()` implementation to prevent missing prerequisites from silently passing.
- `GateLock`: Serializes CI and gate suites via an OS file lock (`flock`) on `target/.tbd-gate.lock`.
- `proc::Run`: Child process execution with process group signal isolation (`killpg`) and concurrent stream draining.

## Code Mapping
- Source: `tools_v2/verification-core/`
