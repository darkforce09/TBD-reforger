# Gate command lines and the ballistics suites

The `gate` and `capture` command lines, and the two gate suites that stay beside them: the
ballistics agreement between the browser and native solvers, and the mortar calculator's offline
pack. Every other gate, the static server and the capture rig are in
`tools/browser_testing/browser_gate_suites`; every browser is driven through
`tools/browser_testing/chrome_devtools_protocol`.

## Contents

```text
tools/developer_tools/src/browser_testing/
├── ballistics_agreement/  `gate ballistics-agreement`: the browser bench's solves against the native solves
├── capture_cli.rs         the `capture` command line: `shot`, `zoomsweep` and `crop`
├── cli.rs                 the `gate` command line and its exit-code mapping
├── mod.rs                 the module tree
├── mortar_offline/        `gate mortar-offline`: the mortar calculator's offline pack and a reload with the server gone
└── tests/                 unit tests for the offline gate and the agreement gate
```

## How it works

```text
bin/gate.rs    ──▶ cli::run ──▶ browser_gate_suites: doctor · v-suite · s-routes · smoke
                                · editor-suite · r-auth · render-check · serve · equipment-data-viewer
                           ──▶ here: mortar-offline · ballistics-agreement
bin/capture.rs ──▶ capture_cli::run ──▶ browser_gate_suites::screen_capture::{shot, zoomsweep, crop}
```

`cli::run` pins the gate font cache before any thread starts, parses with clap, runs the command on
a Tokio runtime, and exits with the code the command returns, or 3 when it returns an error, printed
as `gate: driver error: {error:#}` (every cause of the chain, outermost first). `capture_cli::run`
prints `capture: {error:#}` and exits 2. The command lines stay in this crate because the two suites
below use the ballistics solver of the map engine, which `browser_gate_suites` does not depend on.

| Command | Module | What it asserts | Exit |
|---|---|---|---|
| `gate mortar-offline` | `mortar_offline/` | the first `/tools/mortar` visit stores the offline pack; with the server stopped, the service worker answers the reload, the page is cross-origin isolated, a typed and map-placed fire mission solves to the native solution and the map draws | 0, 1 |
| `gate ballistics-agreement` | `ballistics_agreement/` | the catalog goldens are the committed catalog, and every seeded case the `/debug/ballistics-agreement` bench solves in the browser matches the native solve within 1 mil and 0.1 s; prints a `case ballistics_wasm_agreement_<id> ... ok` line per case and the bit-identical count | 0, 1 |

The other commands and what they assert are listed in the
[browser gate suites](/tools/browser_testing/browser_gate_suites/README.md). Every command also
exits 2 on a clap usage error and 3 on a driver error.

`cargo xtask mk leptos-gates` runs `trunk build --release` in `apps/frontend/`, then
`gate doctor`, `gate editor-suite` and `gate v-suite verify`, each through
`cargo run -q -p developer_tools --bin gate`, stopping at the first failure;
`cargo xtask mk gate-doctor` runs the build and the doctor alone. The `hydrate` smoke in the suite
needs the API on `127.0.0.1:8080`. `cargo xtask mk mortar-offline-gate` runs the build, then
`gate mortar-offline`. `cargo xtask mk ballistics-wasm-agreement` runs the build, then
`gate ballistics-agreement`.

`gate ballistics-agreement` opens its page with `Page::bypass_service_worker`, so the app's
offline service worker never answers in place of the gate's server. `gate mortar-offline` is the
one gate that keeps the worker, because the worker is what it tests.

## Public surface

- `cli::run` and `capture_cli::run`, the entry points of `tools/developer_tools/src/bin/gate.rs`
  and `tools/developer_tools/src/bin/capture.rs`.
- The two suites are public inside the crate for the `gate` command line; nothing else imports them.

## Boundaries

- Depends on: `browser_gate_suites` (every other gate, the static server, the map-asset mounts),
  `chrome_devtools_protocol` (the browser), the map engine's ballistics solver, `clap` and `tokio`.
- Used by: `tools/developer_tools/src/bin/gate.rs` and `capture.rs`; `cargo xtask mk gate-doctor`,
  `cargo xtask mk leptos-gates`, `cargo xtask mk mortar-offline-gate`,
  `cargo xtask mk ballistics-wasm-agreement`, and `.github/workflows/editor-gates.yml`; people, for
  `smoke`, `render-check`, `serve` and `capture`.
- Rules: only the command lines decide the exit codes.

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
