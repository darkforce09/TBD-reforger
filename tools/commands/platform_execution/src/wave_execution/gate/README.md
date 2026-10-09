# Platform wave gate drivers

The two gates of `cargo xtask platform wave gate`: the cheap per-slice gate a slice agent runs in
its worktree before reporting done, and the full wave gate that runs once per
[wave](/documentation/glossary/n_to_z.md#wave) on merged `main`.

## Contents

```text
tools/commands/platform_execution/src/wave_execution/gate/
├── checkrun.rs             step helpers (`checkrun` into the private check folder, `hostrun`) and `gate_slice`
├── clippy_package_sets.rs  the package sets of the wave gate's clippy lanes, derived from the workspace members
└── gate_dispatch.rs        `cmd_gate`: base resolution, the full step list and its verdict
```

## How it works

`tools/commands/platform_execution/src/wave_execution/gate.rs` holds the step `Runner` and the wave
gate's `cargo xtask verify` steps (`VERIFY_STEPS`), derives the wave gate's tool lint packages
(`tool_clippy_packages`: every workspace member under `tools/`; a workspace without xtask or
developer_tools there is red), and re-exports `gate_slice` and `cmd_gate`.
Every step's output is captured. A passing step prints `PASS`; a failing one prints `FAIL` and its
last 15 lines. Every step runs; the gate never stops at the first failure. Both gates take the gate
lock (`super::lock`) and invalidate cargo fingerprints (`super::touch`) before their first cargo
step.

| | Slice gate: `gate --slice <id>` | Wave gate: `gate [<base>]` |
|---|---|---|
| Range | `main...HEAD`; an empty range (run from `main`) refuses with exit 2 | `<base>..HEAD`; the base is derived from the last `wave N CLOSED` commit when omitted and verified by `super::base` |
| Build and style | cargo check, fmt (changed) | fmt (changed), then clippy `-D warnings` in four lanes that together cover every workspace member, derived from the root `Cargo.toml` by `clippy_package_sets.rs`: `clippy native crates` (host target: every member outside `tools/`, the frontend family and the wasm32 members, so the API server with its crates, the game server host agent and every other `crates/**` library), `clippy wasm32 members` (the `wasm-ci` lane's members declaring `targets = "wasm32"` outside the frontend family, for wasm32), `clippy frontend` (`frontend_application` and every `crates/frontend` package, the offline service worker among them, wasm32 and native) and `clippy xtask+developer_tools` (each workspace member under `tools/`) |
| Tests | the frontend family's tests, when changed | `test api` (the API on the gate database), `test frontend` (the frontend family), then `test workspace members`: every other member, derived from the root `Cargo.toml`, one `cargo test --workspace` run excluding the API and frontend families |
| Frontend build | none | trunk build, when the wave touched the frontend's scope |
| Data and contracts | none | `ci verify-codegen-fresh`, then the schema sub-gates of `ci schema-validate` |
| Migrations | none | persist database in advance mode |
| Verifications | none | `VERIFY_STEPS`: crate-tiers, crate-anatomy, tailwind-sources, no-python, no-node, no-shell, file-length |
| Verdict | records `.ai/artifacts/verdicts/<id>.json` for pass and fail | prints `GATE: PASS` or `FAIL` |

A wave-gate step that times out (exit 124 after `TBD_GATE_TIMEOUT`, 1200 s by default) prints
`FAIL (TIMEOUT after <n>s)`. Exit codes: 0 pass; 1 a step failed; 2 a refused base or range, or
a ticket id passed where a base belongs.

## Boundaries

- Depends on: `super::base`, `super::changed`, `super::touch`, `super::db`, `super::migrate`,
  `super::schema`, `super::trunk`, `super::verdict`, `super::lock::GateState` and `super::host`.
- Used by: `tools/commands/platform_execution/src/wave_execution/flush.rs` (the `gate` dispatch) and
  `super::land` (`land` and `wave --close` run the full gate on merged `main`).
- Rules: the slice gate writes its verdict receipt on both pass and fail but never when no step
  ran.

## Related documentation

- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — when each gate
  runs in a wave.
- [Slice agent brief](/documentation/runbooks/factory_waves/slice_agent_brief.md) — the slice
  gate every agent runs before it reports.
