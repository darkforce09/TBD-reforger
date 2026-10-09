# Browser gate suites

The `browser_gate_suites` crate: the headless browser gates of the single-page app. A static server
for the built app, the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) smokes, the gate doctor,
the two ballistics gates (the fire-mission solver's browser build against its native build, and
the mortar calculator's offline pack), the capture rig that photographs the running Mission
Creator, and the `gate` and `capture` command lines that run them all. Every browser is driven
through `chrome_devtools_protocol`.

## Contents

```text
tools/browser_testing/browser_gate_suites/
├── Cargo.toml     the `browser_gate_suites` library package: `chrome_devtools_protocol`, `repository_layout`, `process_runner`, `content_digest`, `newtype_ids`, the ballistics crates, `tokio`, `axum`, `reqwest`, `rustls` (ring), `clap`, `image`, `regex`; layout tier 5
├── gate-env.json  the pinned Chromium build, toolchain and resource floors `gate doctor` checks and `cargo xtask ci ci-chrome` installs from
└── src/           the static server, the editor smokes, the data viewer gate, the ballistics gates, the capture harness, the doctor, the command lines
```

## How it works

```text
bin gate (developer_tools)    ──▶ command_lines::gate::run    ──▶ diagnostics · editor_smoke_tests · server
                                    · equipment_data_viewer
                                    · mortar_offline · ballistics_agreement
bin capture (developer_tools) ──▶ command_lines::capture::run ──▶ screen_capture::{shot, zoomsweep, crop}

every browser gate:  server::start_server(dist) ◀── chrome_devtools_protocol::launch (SwiftShader, 1440×900)
capture:             the running app on :3000   ◀── chrome_devtools_protocol::launch_with_gpu (Vulkan, 1920×1080)
```

The gates render the built app from `crates/frontend/shell/frontend_application/dist` through `server.rs`, which sends
`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: credentialless` and
`Cache-Control: no-store`, answers an extensionless path with `index.html`, streams `/api/`
requests to an optional upstream so an [SSE](/documentation/glossary/n_to_z.md#sse) response arrives
frame by frame, and serves `/map-assets/` from the terrain and glyph folders with byte ranges.

| Command | Module | What it asserts | Exit |
|---|---|---|---|
| `gate doctor` | `diagnostics/` | Chromium, pins, memory, stray processes and fonts, then a 15 s Mission Creator liveness probe | 0, 1 |
| `gate smoke <name>`, `gate editor-suite` | `editor_smoke_tests/` | the Mission Creator smokes, one by name, or the `EDITOR_SUITE` (selfcheck, editor, save-export, undo) in order | 0, 1, 2 |
| `gate r-auth` | `editor_smoke_tests/` | a refused session refreshes exactly once | 0, 1, 2 |
| `gate render-check` | `editor_smoke_tests/` | a path renders, contains `--expect` and passes `--assert-js` | 0, 1 |
| `gate serve` | `server.rs` | none: serves a dist on port 5198 until Ctrl-C | 0 |
| `gate equipment-data-viewer` | `equipment_data_viewer/` | the running website's `/debug/data-viewer` renders, navigates and polls against the imported generation | 0; a failed check exits 3 |
| `gate mortar-offline` | `mortar_offline/` | the first `/tools/mortar` visit stores the offline pack; with the API down and then the server gone, the service worker answers, the page is cross-origin isolated, a typed and map-placed fire mission solves to the native solution and the map draws | 0, 1 |
| `gate ballistics-agreement` | `ballistics_agreement/` | the catalog goldens are the committed catalog, and every seeded case the `/debug/ballistics-agreement` bench solves in the browser matches the native solve | 0, 1 |

Every command also exits 2 on a usage error and 3 on a driver error (an `Error` returned by this
crate, printed as `gate: driver error: ` and the error with every cause). `capture` exits 0 when
the capture is written, 1 when it produced nothing or could not run, and 2 on a usage error.
`fixture_injection.rs` holds `FREEZE_SRC`, the script that makes a `render-check` page
deterministic: it fixes the clock at 1 700 000 000 000 ms, seeds `Math.random` and
`crypto.getRandomValues`, and stops animations.

The doctor's probe, the editor smokes, `render-check` and `r-auth` open their pages
with `Page::bypass_service_worker` (`Network.setBypassServiceWorker`), so the app's offline service
worker never answers in place of the gate's server, its fixtures or its request interception.
`src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo run -q -p developer_tools --bin gate -- doctor   # the gate environment and a liveness probe of the built app
cargo xtask mk leptos-gates                    # build the app, then doctor and editor-suite
cargo xtask mk mortar-offline-gate             # build the app, then gate mortar-offline
cargo xtask mk ballistics-wasm-agreement       # build the app, then gate ballistics-agreement
```

The crate has no unit tests; its gates are the tests, run against the built app.

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `CHROME_HEADLESS_SHELL`, `PLAYWRIGHT_BROWSERS_PATH` | the Playwright cache | which Chromium the gates launch (see `chrome_devtools_protocol`) |
| `TBD_GATE_FONT_CACHE` | a gate-owned folder | the fontconfig cache the gates' browsers read; `gate doctor` reports and installs it |

## Public surface

The modules `ballistics_agreement`, `diagnostics`, `editor_smoke_tests`,
`equipment_data_viewer`, `fixture_injection`, `gate_layout`, `mortar_offline`, `screen_capture`, `server` and `session_tokens`, each entry returning this crate's `Result`;
`command_lines::gate::run` and `command_lines::capture::run`, the whole of the `gate` and
`capture` binaries; `Error` (with `Error::with_causes`, the printed chain) and `Result` at the
root; the `prelude` module re-exports the server and the map-asset mounts.

## Boundaries

- Depends on: `chrome_devtools_protocol` (every browser); `repository_root` (the checkout root,
  from the working directory); `repository_layout` (the map asset and glyph folders and the gate
  pin); `process_runner`
  (the toolchain version probes of the doctor); `content_digest` (the catalog digests);
  `newtype_ids` (the captured mission's id); `ballistics_model`, `fire_mission_planning` and
  `ballistics_agreement_cases` (the native solves and the seeded cases of the ballistics gates);
  `map_coordinates` (the grid references the offline mortar gate types); `tokio`, `axum`,
  `reqwest`, `rustls` (the ring provider), `url`, `clap`, `image`, `regex`, `serde`, `serde_json`, `base64`, `futures-util`,
  `libc`, `thiserror`; the built app in `crates/frontend/shell/frontend_application/dist`, the fixtures and the committed
  catalog in `contracts/`.
- Used by: the `gate` and `capture` binaries of `tools/developer_tools/src/bin/`, one call each
  into `command_lines`; through them, `cargo xtask mk gate-doctor`, `cargo xtask mk leptos-gates`,
  `cargo xtask mk mortar-offline-gate`, `cargo xtask mk ballistics-wasm-agreement` and
  `.github/workflows/editor-gates.yml`.
- Rules:
  - tier 5 of `tools/browser_testing`; a gate's verdict is its exit code, an error means it could
    not run, only `command_lines` turns either into the process exit code, and nothing calls
    `std::process::exit`;
  - the API proxy streams and `RunningServer::close` never waits on an open stream;
  - every harness token names the one gate session;
  - the gates run on SwiftShader, so they need no GPU; only the capture rig uses the real device.

## Related documentation

- [Chrome DevTools Protocol client](/tools/browser_testing/chrome_devtools_protocol/README.md) —
  the browser launch and the page calls.
- [Gate command lines](/tools/browser_testing/browser_gate_suites/src/command_lines/README.md) — the
  `gate` and `capture` dispatch and exit codes.
- [Offline mortar page](/documentation/runbooks/offline_mortar_page.md) — the offline gate's
  procedure.
- [Editor gates](/documentation/runbooks/editor_gates.md) — running the gates, their environment
  and the wedge modes.
- [Editor capture](/documentation/runbooks/editor_capture.md) — capturing a running Mission
  Creator.
