# Gate command lines

The `gate` and `capture` command lines of the `browser_gate_suites` crate: they parse the
arguments, run one gate or capture driver of the crate, and turn its outcome into the process exit
code. The `gate` and `capture` binaries of `developer_tools` are one call each into this module.

## Contents

```text
tools/browser_testing/browser_gate_suites/src/command_lines/
├── capture.rs  the `capture` command line: `shot`, `zoomsweep` and `crop`
└── gate.rs     the `gate` command line: every gate subcommand and its exit-code mapping
```

## How it works

```text
bin/gate.rs    ──▶ gate::run    ──▶ diagnostics · dom_oracle · route_drift · editor_smoke_tests
                                    · server · equipment_data_viewer · mortar_offline
                                    · ballistics_agreement
bin/capture.rs ──▶ capture::run ──▶ screen_capture::{shot, zoomsweep, crop}
```

`gate::run` pins the gate font cache before any thread starts, parses with clap, runs the command
on a Tokio runtime, and exits with the code the command returns, or 3 when it returns an error,
printed as `gate: driver error: ` and `Error::with_causes` (every cause of the chain, outermost
first). `capture::run` prints `capture: ` and the error with its causes and exits 1. The
commands and what each asserts are listed in the
[browser gate suites](/tools/browser_testing/browser_gate_suites/README.md). Every command also
exits 2 on a clap usage error.

`cargo xtask mk leptos-gates` runs `trunk build --release` in `crates/frontend/shell/frontend_application/`, then
`gate doctor`, `gate editor-suite` and `gate v-suite verify`, each through
`cargo run -q -p developer_tools --bin gate`, stopping at the first failure;
`cargo xtask mk gate-doctor` runs the build and the doctor alone. The `hydrate` smoke in the suite
needs the API on `127.0.0.1:8080`. `cargo xtask mk mortar-offline-gate` runs the build, then
`gate mortar-offline`. `cargo xtask mk ballistics-wasm-agreement` runs the build, then
`gate ballistics-agreement`.

## Public surface

- `gate::run` and `capture::run`, the whole of `tools/developer_tools/src/bin/gate.rs` and
  `tools/developer_tools/src/bin/capture.rs`.

## Boundaries

- Depends on: the gates and drivers of this crate, `clap` and `tokio`.
- Used by: `tools/developer_tools/src/bin/gate.rs` and `capture.rs`; `cargo xtask mk gate-doctor`,
  `cargo xtask mk leptos-gates`, `cargo xtask mk mortar-offline-gate`,
  `cargo xtask mk ballistics-wasm-agreement`, and `.github/workflows/editor-gates.yml`; people, for
  `smoke`, `render-check`, `serve` and `capture`.
- Rules: only the command lines decide the exit codes; neither calls `std::process::exit`.

## Related documentation

- [Developer tool executables](/tools/developer_tools/src/bin/README.md) — the `gate` and
  `capture` synopses and exit codes.
- [Browser gate suites](/tools/browser_testing/browser_gate_suites/README.md) — the gates, the
  static server and the capture rig.
- [Editor gates](/documentation/runbooks/editor_gates.md) — running the gates, their environment
  and the wedge modes.
- [Offline mortar page](/documentation/runbooks/offline_mortar_page.md) — the offline gate's
  procedure.
- [Build and development-server commands](/tools/commands/ci_task_catalog/src/build_lane/README.md) —
  `cargo xtask mk`, including `gate-doctor` and `leptos-gates`.
