# Chrome DevTools Protocol client

The `chrome_devtools_protocol` crate: the client the headless browser gates drive Chromium with. It
finds a Chromium build, starts it headless in its own process group with the gate's GPU flags and
its own fontconfig cache, opens pages over one WebSocket each, and speaks the protocol on them:
calls, events, navigation, evaluation, trusted input, screenshots and request interception.

## Contents

```text
tools/browser_testing/chrome_devtools_protocol/
├── Cargo.toml  the `chrome_devtools_protocol` library package: `newtype_ids`, `tokio`, `tokio-tungstenite`, `reqwest`, `futures-util`, `serde_json`, `base64`, `libc`, `thiserror`; layout tier 1
└── src/        the browser and page handles, Chromium discovery, the launch and page setup, the gate font cache, the error type
```

## How it works

```text
find_chromium()  CHROME_HEADLESS_SHELL ─▶ PLAYWRIGHT_BROWSERS_PATH ─▶ ~/.cache/ms-playwright
      │
launch(port, args) = launch_with_gpu(port, args, GpuBackend::Swiftshader)
      │   --headless=new (not for the headless shell), --remote-debugging-port, a fresh profile,
      │   the backend's GPU flags; XDG_CACHE_HOME = gate_font_cache_dir(); own process group
      ▼
Browser ──▶ new_page(&browser, url, init_scripts) ──▶ Page
                1440×900 viewport and init scripts first, then the navigation
Page::send / evaluate / wait_for / navigate / dispatch_mouse / dispatch_key / screenshot /
     fulfill_json / fulfill_raw / continue_request / bypass_service_worker / close
Browser::shutdown: SIGTERM the group, reap (5 s), SIGKILL the group, remove the profile
```

`src/README.md` describes each module and the launch in detail.

## Getting started

Run from the repository root:

```bash
cargo test -p chrome_devtools_protocol   # Chromium discovery over scratch Playwright folders
```

The tests build scratch folders and need no browser. The gates that launch one are run through
the `gate` binary (see [Editor gates](/documentation/runbooks/editor_gates.md)).

## Configuration

| Variable | Default | Effect |
|---|---|---|
| `CHROME_HEADLESS_SHELL` | unset | the Chromium executable, when it names an existing file |
| `PLAYWRIGHT_BROWSERS_PATH` | `~/.cache/ms-playwright` | the Playwright browser folder searched next |
| `TBD_GATE_FONT_CACHE` | a gate-owned folder under the system temporary folder | the fontconfig cache every launched browser reads; empty counts as unset |

## Public surface

At the crate root: `Browser`, `Page`, `GpuBackend`, `VIEWPORT`, `launch`, `launch_with_gpu`,
`new_page`, `sleep_ms`, `wait_http`, `InterceptedRequestId`, `find_chromium`, `is_headless_shell`,
`CacheOrigin`,
`GATE_FONT_CACHE_ENV`, `gate_font_cache_dir`, `resolved_font_cache`, `Error` and `Result`; the
`prelude` module re-exports the common handles.

## Boundaries

- Depends on: `newtype_ids` (the typed request id); `tokio` (the browser child, its output
  drains, the page sockets), `tokio-tungstenite`, `reqwest` (the debugging endpoint), `futures-util`,
  `serde_json`, `base64`, `libc` (process-group signals) and `thiserror`.
- Used by: `tools/browser_testing/browser_gate_suites`, every gate and the capture harness in it.
- Rules: tier 1 of `tools/browser_testing`; Chromium is spawned through `tokio::process`, because
  its output is drained by tasks for the browser's whole life, which the synchronous
  `process_runner` cannot host; both output pipes are drained from spawn; the viewport and the
  init scripts are applied before the first navigation; every launch gets its own profile folder.

## Related documentation

- [Browser gate suites](/tools/browser_testing/browser_gate_suites/README.md) — the gates that
  drive this client.
- [Editor gates](/documentation/runbooks/editor_gates.md) — the required Chromium build and the
  wedge modes the launch settings avoid.
