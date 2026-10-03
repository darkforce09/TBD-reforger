# Browser gate suites

The `browser_gate_suites` crate: the headless browser gates of the single-page app. A static server
for the built app, and the gates that hold it to its frozen DOM, its route table and the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) smokes, plus the gate doctor
and the capture rig that photographs the running Mission Creator. Every browser is driven through
`chrome_devtools_protocol`.

## Contents

```text
tools/browser_testing/browser_gate_suites/
├── Cargo.toml     the `browser_gate_suites` library package: `chrome_devtools_protocol`, `repository_layout`, `process_runner`, `content_digest`, `newtype_ids`, `tokio`, `axum`, `reqwest`, `image`, `regex`; layout tier 2
├── fixtures/      the DOM oracle goldens and the committed route table the gates compare against
├── gate-env.json  the pinned Chromium build, toolchain and resource floors `gate doctor` checks and `cargo xtask ci ci-chrome` installs from
└── src/           the static server, the DOM oracle, the route drift check, the editor smokes, the data viewer gate, the capture harness, the doctor
```

## How it works

```text
gate (developer_tools) ──▶ diagnostics · dom_oracle · route_drift · editor_smoke_tests
                           · server · equipment_data_viewer
capture (developer_tools) ──▶ screen_capture::{shot, zoomsweep, crop}

every browser gate:  server::start_server(dist) ◀── chrome_devtools_protocol::launch (SwiftShader, 1440×900)
capture:             the running app on :3000   ◀── chrome_devtools_protocol::launch_with_gpu (Vulkan, 1920×1080)
```

The gates render the built app from `apps/frontend/dist` through `server.rs`, which sends
`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: credentialless` and
`Cache-Control: no-store`, answers an extensionless path with `index.html`, streams `/api/`
requests to an optional upstream so an [SSE](/documentation/glossary/n_to_z.md#sse) response arrives
frame by frame, and serves `/map-assets/` from the terrain and glyph folders with byte ranges.

| Command | Module | What it asserts | Exit |
|---|---|---|---|
| `gate doctor` | `diagnostics/` | Chromium, pins, memory, stray processes and fonts, then a 15 s Mission Creator liveness probe | 0, 1 |
| `gate v-suite verify` | `dom_oracle/` | each of 26 routes' normalised DOM equals its golden, with every [API](/documentation/glossary/a_to_f.md#api) call fed from fixtures | 0, 1, 2 |
| `gate v-suite accept` | `dom_oracle/` | replaces one route's golden, with a note | 0, 2 |
| `gate s-routes` | `route_drift.rs` | the `ROUTES` table of `apps/frontend/src/foundation/route_table/mod.rs` equals `manifests/routes.csv` | 0, 1 |
| `gate smoke <name>`, `gate editor-suite` | `editor_smoke_tests/` | the Mission Creator smokes, one or all in `EDITOR_SUITE` order | 0, 1, 2 |
| `gate r-auth` | `editor_smoke_tests/` | a refused session refreshes exactly once | 0, 1, 2 |
| `gate render-check` | `editor_smoke_tests/` | a path renders, contains `--expect` and passes `--assert-js` | 0, 1 |
| `gate serve` | `server.rs` | none: serves a dist on port 5198 until Ctrl-C | 0 |
| `gate equipment-data-viewer` | `equipment_data_viewer/` | the running website's `/debug/data-viewer` renders, navigates and polls against the imported generation | 0; a failed check exits 3 |

Every command also exits 2 on a usage error and 3 on a driver error (an `Error` returned by this
crate). `gate s-routes` writes the router's rows as `path,component,fullBleed,chromeless,router_auth`,
sorted by path, and prints each differing row. `fixture_injection.rs` holds the two scripts that
make a capture deterministic: `FREEZE_SRC` fixes the clock at 1 700 000 000 000 ms, seeds
`Math.random` and `crypto.getRandomValues`, and stops animations; `DOM_SERIALIZER_SRC` defines
`window.__domOracleSerialize`, which the goldens were serialised with.

The doctor's probe, the editor smokes, `render-check`, `r-auth` and the DOM oracle open their pages
with `Page::bypass_service_worker` (`Network.setBypassServiceWorker`), so the app's offline service
worker never answers in place of the gate's server, its fixtures or its request interception.
`src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p browser_gate_suites             # the fixture router, accept floor, payload pins, server, API corpus, tokens
cargo run -q -p developer_tools --bin gate -- doctor   # the gate environment and a liveness probe of the built app
cargo xtask mk leptos-gates                    # build the app, then doctor, editor-suite and v-suite verify
```

The unit tests start the static server on loopback ports and need no browser.

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `CHROME_HEADLESS_SHELL`, `PLAYWRIGHT_BROWSERS_PATH` | the Playwright cache | which Chromium the gates launch (see `chrome_devtools_protocol`) |
| `TBD_GATE_FONT_CACHE` | a gate-owned folder | the fontconfig cache the gates' browsers read; `gate doctor` reports and installs it |

## Public surface

The modules `diagnostics`, `dom_oracle`, `editor_smoke_tests`, `equipment_data_viewer`,
`fixture_injection`, `gate_layout`, `route_drift`, `screen_capture`, `server` and
`session_tokens`, each entry returning this crate's `Result`; `Error` and `Result` at the root; the
`prelude` module re-exports the server and the map-asset mounts.

## Boundaries

- Depends on: `chrome_devtools_protocol` (every browser); `repository_layout` (the checkout root,
  from the working directory, the map asset and glyph folders and the gate pin); `process_runner`
  (the toolchain version probes of the doctor); `content_digest` (the DOM digests);
  `newtype_ids` (the captured mission's id); `tokio`, `axum`, `reqwest`, `url`, `image`, `regex`,
  `serde_json`, `base64`, `futures-util`, `libc`, `thiserror`; the built app in
  `apps/frontend/dist`, the fixtures in `contracts/fixtures/api_goldens/` and the goldens in
  `fixtures/dom_oracle/`.
- Used by: the `gate` and `capture` command lines in `tools/developer_tools/src/browser_testing/`
  (`cli.rs`, `capture_cli.rs`), and the offline mortar and ballistics agreement suites there (the
  static server); through the binaries, `cargo xtask mk gate-doctor`, `cargo xtask mk leptos-gates`
  and `.github/workflows/editor-gates.yml`.
- Rules:
  - tier 2 of `tools/browser_testing`; a gate's verdict is its exit code, an error means it could
    not run, and no gate calls `std::process::exit`;
  - `FREEZE_SRC` and `DOM_SERIALIZER_SRC` are the exact bytes the goldens were captured with, and
    their SHA-256 is pinned (`payloads_are_pinned` in
    `src/tests/fixture_injection/tests.rs`); a change re-pins both and re-accepts every affected
    golden;
  - the API proxy streams and `RunningServer::close` never waits on an open stream
    (`sse_frames_arrive_incrementally_not_buffered`, `close_does_not_hang_on_a_still_open_stream`);
  - every harness token names the one gate session (`every_rotation_names_the_same_gate_session`);
  - the gates run on SwiftShader, so they need no GPU; only the capture rig uses the real device.

## Related documentation

- [Chrome DevTools Protocol client](/tools/browser_testing/chrome_devtools_protocol/README.md) —
  the browser launch and the page calls.
- [Gate command lines](/tools/developer_tools/src/browser_testing/README.md) — the `gate` and
  `capture` dispatch and the two suites that stay with them.
- [Editor gates](/documentation/runbooks/editor_gates.md) — running the gates, their environment
  and the wedge modes.
- [Editor capture](/documentation/runbooks/editor_capture.md) — capturing a running Mission
  Creator.
- [DOM oracle fixtures](/tools/browser_testing/browser_gate_suites/fixtures/dom_oracle/README.md) —
  the goldens and the route table the gates compare against.
