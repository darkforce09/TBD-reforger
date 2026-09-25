# Headless browser gates and captures

The code behind the `gate` and `capture` binaries: a DevTools protocol client that drives headless
Chromium, a static server for the built single-page app, and the gates that hold the app to its
frozen DOM, its route table and the [Mission Creator](/documentation_v2/glossary.md#mission-creator)
smokes, plus the capture rig that photographs the running Mission Creator.

## Contents

```text
tools_v2/developer-tools/src/browser_testing/
├── capture_cli.rs         the `capture` command line: `shot`, `zoomsweep` and `crop`
├── cdp/                   Chromium discovery and launch, page setup and polling helpers
├── cdp.rs                 `Browser`, `Page` and `GpuBackend`: the DevTools protocol client
├── cli.rs                 the `gate` command line and its exit-code mapping
├── diagnostics/           the gate font cache and the `gate doctor` checks
├── diagnostics.rs         `gate doctor`'s types and constants; re-exports its entry points
├── dom_oracle/            the `gate v-suite` routes, request router and verify and accept modes
├── dom_oracle.rs          the DOM oracle's types and the accept size floor; re-exports its entries
├── editor_smoke_tests/    the Mission Creator smokes and the render-check, session and perf gates
├── editor_smoke_tests.rs  the shared smoke `Harness`, `EDITOR_SUITE` and the perf probe scripts
├── fixture_injection.rs   `FREEZE_SRC` and `DOM_SERIALIZER_SRC`, the scripts injected into each page
├── mod.rs                 the module tree
├── route_drift.rs         `gate s-routes`: the router's route table against the committed CSV
├── screen_capture/        the `shot`, `zoomsweep` and `crop` drivers
├── screen_capture.rs      the capture viewport, debug port, overlay selector and blank-canvas floor
├── server.rs              the static server: isolation headers, SPA fallback, API proxy, map assets
├── session_tokens.rs      the unsigned access tokens the harness answers a token refresh with
└── tests/                 unit tests for the fixture router, accept floor, payload pins, server, tokens
```

## How it works

```text
bin/gate.rs    ──▶ cli::run ──▶ doctor · v-suite · s-routes · smoke · editor-suite · r-auth
                                · render-check · serve
bin/capture.rs ──▶ capture_cli::run ──▶ screen_capture::{shot, zoomsweep, crop}

every browser gate:  server::start_server(dist) ◀── cdp::launch (SwiftShader, 1440×900)
capture:             the running app on :3000   ◀── cdp::launch_with_gpu (Vulkan, 1920×1080)
```

`cli::run` pins the gate font cache before any thread starts, parses with clap, runs the command on
a Tokio runtime, and exits with the code the command returns, or 3 when it returns an error. The
gates render the built app from `apps/website/frontend/dist` through `server.rs`, which sends
`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: credentialless` and
`Cache-Control: no-store`, answers an extensionless path with `index.html`, streams `/api/`
requests to an optional upstream so an [SSE](/documentation_v2/glossary.md#sse) response arrives
frame by frame, and serves `/map-assets/` from the terrain and glyph folders with byte ranges.

| Command | Module | What it asserts | Exit |
|---|---|---|---|
| `gate doctor` | `diagnostics/` | Chromium, pins, memory, stray processes and fonts, then a 15 s Mission Creator liveness probe | 0, 1 |
| `gate v-suite verify` | `dom_oracle/` | each of 25 routes' normalised DOM equals its golden, with every [API](/documentation_v2/glossary.md#api) call fed from fixtures | 0, 1, 2 |
| `gate v-suite accept` | `dom_oracle/` | replaces one route's golden, with a note | 0, 2 |
| `gate s-routes` | `route_drift.rs` | the `ROUTES` table of `apps/website/frontend/src/router.rs` equals `manifests/routes.csv` | 0, 1 |
| `gate smoke <name>`, `gate editor-suite` | `editor_smoke_tests/` | the Mission Creator smokes, one or all in `EDITOR_SUITE` order | 0, 1, 2 |
| `gate r-auth` | `editor_smoke_tests/` | a refused session refreshes exactly once | 0, 1, 2 |
| `gate render-check` | `editor_smoke_tests/` | a path renders, contains `--expect` and passes `--assert-js` | 0, 1 |
| `gate serve` | `server.rs` | none: serves a dist on port 5198 until Ctrl-C | 0 |

Every command also exits 2 on a clap usage error and 3 on a driver error. `gate s-routes` writes
the router's rows as `path,component,fullBleed,chromeless,router_auth`, sorted by path, and prints
each differing row. The DOM goldens and route CSV live in
`tools_v2/developer-tools/fixtures/dom_oracle/`. `fixture_injection.rs` holds the two scripts that
make a capture deterministic: `FREEZE_SRC` fixes the clock at 1 700 000 000 000 ms, seeds
`Math.random` and `crypto.getRandomValues`, and stops animations; `DOM_SERIALIZER_SRC` defines
`window.__domOracleSerialize`, which the goldens were serialised with.

`cargo xtask mk leptos-gates` runs `trunk build --release` in `apps/website/frontend/`, then
`gate doctor`, `gate editor-suite` and `gate v-suite verify`, each through
`cargo run -q -p developer-tools --bin gate`, stopping at the first failure;
`cargo xtask mk gate-doctor` runs the build and the doctor alone. The `hydrate` smoke in the suite
needs the API on `127.0.0.1:8080`.

## Public surface

- `cli::run` and `capture_cli::run`, the entry points of `tools_v2/developer-tools/src/bin/gate.rs`
  and `tools_v2/developer-tools/src/bin/capture.rs`.
- `server::repo_root`, the checkout root the Enfusion MCP broker in
  `tools_v2/developer-tools/src/enfusion_tooling/mcp_broker.rs` resolves its runner from.
- The other modules are public inside the crate for the binaries and each other; nothing else
  imports them.

## Boundaries

- Depends on: `crate::repository_layout` for the map asset and glyph folders; `tokio`, `axum`,
  `reqwest`, `tokio-tungstenite`, `clap`, `image` and `sha2`; a Chromium build from the Playwright
  cache or `CHROME_HEADLESS_SHELL`; the pins in `tools_v2/developer-tools/gate-env.json`; the built
  app in `apps/website/frontend/dist`, the fixtures in `apps/website/frontend/tests/fixtures/api/`
  and the goldens in `tools_v2/developer-tools/fixtures/dom_oracle/`.
- Used by: `tools_v2/developer-tools/src/bin/gate.rs` and `capture.rs`; `repo_root` in
  `tools_v2/developer-tools/src/enfusion_tooling/mcp_broker.rs`; `cargo xtask mk gate-doctor` and
  `cargo xtask mk leptos-gates`, and `.github/workflows/editor-gates.yml`, which also installs the
  pinned Chromium with `cargo xtask ci ci-chrome`; people, for `smoke`, `render-check`, `serve` and
  `capture`.
- Rules:
  - `FREEZE_SRC` and `DOM_SERIALIZER_SRC` are the exact bytes the goldens were captured with, and
    their SHA-256 is pinned (`payloads_are_pinned` in
    `tools_v2/developer-tools/src/browser_testing/tests/fixture_injection/tests.rs`); a change
    re-pins both and re-accepts every affected golden;
  - the API proxy streams and `RunningServer::close` never waits on an open stream
    (`sse_frames_arrive_incrementally_not_buffered`, `close_does_not_hang_on_a_still_open_stream`
    in `tools_v2/developer-tools/src/browser_testing/tests/server/tests.rs`);
  - every harness token names the one gate session (`every_rotation_names_the_same_gate_session`
    in `tools_v2/developer-tools/src/browser_testing/tests/session_tokens.rs`);
  - the gates run on SwiftShader, so they need no GPU; only the capture rig uses the real device.

## Related documentation

- [Developer tool executables](/tools_v2/developer-tools/src/bin/README.md) — the `gate` and
  `capture` synopses and exit codes.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running the gates, their environment
  and the wedge modes.
- [Editor capture](/documentation_v2/runbooks/editor_capture.md) — capturing a running Mission
  Creator.
- [DOM oracle fixtures](/tools_v2/developer-tools/fixtures/dom_oracle/README.md) — the goldens and
  the route table the gates compare against.
- [Build and development-server commands](/tools_v2/xtask/src/commands/build/README.md) —
  `cargo xtask mk`, including `gate-doctor` and `leptos-gates`.
