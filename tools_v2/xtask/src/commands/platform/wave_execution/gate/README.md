# Platform wave gate drivers

The two gates of `cargo xtask platform wave gate`: the cheap per-slice gate a slice agent runs in
its worktree before reporting done, and the full wave gate that runs once per
[wave](/documentation_v2/glossary.md#wave) on merged `main`.

## Contents

```text
tools_v2/xtask/src/commands/platform/wave_execution/gate/
├── checkrun.rs       step helpers (`checkrun` into the private check folder, `hostrun`) and `gate_slice`
└── gate_dispatch.rs  `cmd_gate`: base resolution, the full step list and its verdict
```

## How it works

`tools_v2/xtask/src/commands/platform/wave_execution/gate.rs` holds the step `Runner` and the ten
shared `cargo xtask verify` steps (`VERIFY_STEPS`), and re-exports `gate_slice` and `cmd_gate`.
Every step's output is captured. A passing step prints `PASS`; a failing one prints `FAIL` and its
last 15 lines. Every step runs; the gate never stops at the first failure. Both gates take the gate
lock (`super::lock`) and invalidate cargo fingerprints (`super::touch`) before their first cargo
step.

| | Slice gate: `gate --slice <id>` | Wave gate: `gate [<base>]` |
|---|---|---|
| Range | `main...HEAD`; an empty range (run from `main`) refuses with exit 2 | `<base>..HEAD`; the base is derived from the last `wave N CLOSED` commit when omitted and verified by `super::base` |
| Build and style | cargo check, wasm32 (frontend), fmt (changed), clippy (changed crates) | cargo check, wasm32 (frontend), fmt (changed), clippy for the API, map engine, frontend, xtask and developer-tools |
| Tests | frontend tests, when changed | API (on the gate database), map engine with all features, frontend, xtask and developer-tools |
| Frontend build | none | trunk build, when the wave touched the frontend's scope |
| Data and contracts | schema, catalogue drift (`world reclassify --terrain everon`) | the same, plus ticket registry and wave lock |
| Migrations | claim body; persist database in audit mode | claim body; persist database in advance mode |
| Verifications | the ten `VERIFY_STEPS`, `no-python` | the ten `VERIFY_STEPS`, `no-python`, `no-node`, `no-shell`, `ci-shell` |
| Verdict | records `.ai/artifacts/verdicts/<id>.json` for pass and fail | prints `GATE: PASS` or `FAIL` |

A wave-gate step that times out (exit 124 after `TBD_GATE_TIMEOUT`, 1200 s by default) prints
`FAIL (TIMEOUT after <n>s)`. Exit codes: 0 pass; 1 a step failed; 2 a refused base or range, or
a ticket id passed where a base belongs.

## Boundaries

- Depends on: `super::base`, `super::changed`, `super::touch`, `super::db`, `super::migrate`,
  `super::schema`, `super::trunk`, `super::verdict`, `super::lock::GateState` and `super::host`.
- Used by: `tools_v2/xtask/src/commands/platform/wave_execution/flush.rs` (the `gate` dispatch) and
  `super::land` (`land` and `wave --close` run the full gate on merged `main`).
- Rules: both gates iterate the one `VERIFY_STEPS` table, so no verification is wired into only
  one of them, and `cargo xtask verify ci-schema-parity` fails when a row disappears; the slice
  gate writes its verdict receipt on both pass and fail but never when no step ran; the tests are
  in `tools_v2/xtask/src/commands/platform/wave_execution/tests/gate/tests.rs`.
