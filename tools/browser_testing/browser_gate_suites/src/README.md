# Browser gate suites source

The modules of the `browser_gate_suites` crate: the static server, the gates of the single-page
app, the capture harness, the doctor, the scripts, tokens and locations they share, and the `gate`
and `capture` command lines.

## Contents

```text
tools/browser_testing/browser_gate_suites/src/
├── ballistics_agreement/  `gate ballistics-agreement`: the browser bench's solves against the native solves
├── command_lines/         the `gate` and `capture` command lines and their exit codes
├── command_lines.rs       the command lines' module tree
├── diagnostics/           the gate font cache and the `gate doctor` checks
├── diagnostics.rs         `gate doctor`'s types and constants; re-exports its entry points
├── dom_oracle/            the `gate v-suite` routes, request router and verify and accept modes
├── dom_oracle.rs          the DOM oracle's types and the accept size floor; re-exports its entries
├── editor_smoke_tests/    the Mission Creator smokes and the render-check, session and perf gates
├── editor_smoke_tests.rs  the shared smoke `Harness`, `EDITOR_SUITE` and the perf probe scripts
├── equipment_data_viewer/  `gate equipment-data-viewer`: the live equipment data viewer check
├── error.rs               `Error` and `Result`: why a gate, the server or the capture harness could not run
├── fixture_injection.rs   `FREEZE_SRC` and `DOM_SERIALIZER_SRC`, the scripts injected into each page
├── gate_layout.rs         `MapAssetMounts` and the editor gate runbook: the locations only the gates name
├── lib.rs                 the crate root: module header, `mod` lines and the re-exports
├── mortar_offline/        `gate mortar-offline`: the mortar calculator's offline pack and a reload with the server gone
├── prelude.rs             the static server and the map-asset mounts for glob import
├── route_drift.rs         `gate s-routes`: the router's route table against the committed CSV
├── screen_capture/        the `shot`, `zoomsweep` and `crop` drivers
├── screen_capture.rs      the capture viewport, debug port, overlay selector and blank-canvas floor
├── server/                the recorded API corpus route of the static server
├── server.rs              the static server: isolation headers, SPA fallback, API proxy, API corpus, map assets
├── session_tokens.rs      the unsigned access tokens and Bearer token-pair answers of a token refresh
└── tests/                 unit tests for the fixture router, accept floor, payload pins, smoke assertions, server, API corpus, tokens, gate layout, the ballistics gates
```

## How it works

Each gate is one async entry that builds its own server and browser, runs its checks, prints its
verdict and returns its exit code (0 green, 1 gate fail, 2 a usage or scenario error); an `Error`
means the gate could not run, and the `gate` command line maps it to exit 3. An `Error::Context`
displays its step alone and carries the failure underneath as its source, so
`Error::with_causes` (the command lines' text, and the failure line of the two ballistics gates)
prints `step: cause`, while a verdict that records one error (`{error}`) keeps the outermost step
only. The folders' own READMEs (`dom_oracle/`, `editor_smoke_tests/`, `diagnostics/`,
`screen_capture/`, `server/`, `equipment_data_viewer/`, `mortar_offline/`,
`ballistics_agreement/`, `command_lines/`) describe their gates.

## Boundaries

- Depends on: `chrome_devtools_protocol`, `repository_layout`, `process_runner`, `content_digest`,
  `newtype_ids`, `ballistics_model`, `fire_mission_planning`, `ballistics_agreement_cases`,
  `map_coordinates`, `tokio`, `axum`, `reqwest`, `url`, `clap`, `image`, `regex`, `serde`,
  `serde_json`, `base64`, `futures-util`, `libc`, `thiserror`.
- Used by: `lib.rs`; through it, the `gate` and `capture` binaries of
  `tools/developer_tools/src/bin/`.
- Rules: no module calls `std::process::exit`; the doctor's font probe and the static server run
  on tokio, the doctor's version probes go through `process_runner`.
