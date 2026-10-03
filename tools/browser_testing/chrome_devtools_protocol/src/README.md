# Chrome DevTools Protocol client source

The modules of the `chrome_devtools_protocol` crate: the browser and page handles, finding a
Chromium build, starting it headless with the right GPU flags, draining its output, opening a
page, the gate's own font cache, and the small polling helpers every gate uses.

## Contents

```text
tools/browser_testing/chrome_devtools_protocol/src/
├── browser_launch.rs      `sleep_ms`, `launch`, `launch_with_gpu`, `new_page`, `wait_http`, the pipe drain and the event-field merge; a child module of `browser_session.rs`
├── browser_session.rs     `Browser`, `Page`, `GpuBackend` and `VIEWPORT`: the process group, the page socket and every protocol call
├── chromium_discovery.rs  `find_chromium` and `is_headless_shell`: which Chromium executable the gates launch
├── error.rs               `Error` and `Result`: why a launch, a page setup or a protocol call failed
├── gate_font_cache.rs     `gate_font_cache_dir`, `resolved_font_cache` and `TBD_GATE_FONT_CACHE`: the fontconfig cache folder every launch reads
├── intercepted_request.rs  `InterceptedRequestId`: the typed `requestId` of a paused request a fulfil or continue call answers
├── lib.rs                 the crate root: module header, `mod` lines and the re-exports
├── prelude.rs             the common handles for glob import
└── tests/                 unit tests for Chromium discovery over scratch Playwright folders
```

## How it works

`find_chromium` takes `CHROME_HEADLESS_SHELL` when it names an existing file, and otherwise
searches the Playwright browser folder `PLAYWRIGHT_BROWSERS_PATH`, then `~/.cache/ms-playwright`;
the first folder holding a known layout wins. Inside a folder a full build
(`chromium-*/chrome-linux64/chrome` or `chromium-*/chrome-linux/chrome`) beats a headless shell
(`chromium_headless_shell-*/chrome-headless-shell-linux64/chrome-headless-shell` or
`chromium_headless_shell-*/chrome-linux/headless_shell`), because the headless shell aborts on
per-character font fallback; among builds of one kind the highest build number wins, compared as a
number.
`launch` is `launch_with_gpu` with `GpuBackend::Swiftshader`, the software WebGL2 path every gate
uses; the capture harness passes `GpuBackend::Vulkan` instead. Each launch:

- adds `--headless=new` unless the binary is the headless shell, then `--no-sandbox`,
  `--remote-debugging-port`, a fresh profile directory `tbd-cdp-<pid>-<port>` under the temporary
  directory, the backend's GPU flags, `--enable-unsafe-webgpu`, `--hide-scrollbars` and
  `--force-device-scale-factor=1`;
- sets `XDG_CACHE_HOME` on the child to the gate's own font cache, `gate_font_cache_dir`;
- starts Chromium in its own process group, so `Browser::shutdown` can signal the whole tree;
- drains stdout and stderr from spawn (`drain_pipe`), keeping the last 200 lines for
  `Browser::recent_output`, because an undrained 64 KiB pipe blocks whichever Chromium thread
  writes next;
- polls `/json/version` up to 80 times, 125 ms apart.

`new_page` opens a target through `/json/new`, connects its WebSocket, enables the page and
runtime domains, applies the 1440×900 viewport, registers the init scripts with
`Page.addScriptToEvaluateOnNewDocument`, and only then navigates when given a URL. `wait_http`
polls a URL until it answers 2xx or 404, and `merge` folds extra fields into a mouse or key event.

`browser_launch.rs` is declared by `browser_session.rs` (not by `lib.rs`) because it builds the
handles' private fields; every public function of it is re-exported at the crate root.
An `Error::Context` displays its step alone and carries the failure underneath as its source, so a
chain walk prints `step: cause`, as the gates' `{error:#}` output always has.

## Boundaries

- Depends on: `newtype_ids` (the request id), `tokio`, `reqwest`, `tokio-tungstenite`,
  `futures-util`, `serde_json`, `base64`, `libc` and `thiserror`.
- Used by: `lib.rs`, which re-exports every public item; through it, every gate and the capture
  harness in `tools/browser_testing/browser_gate_suites/`, the ballistics agreement and offline
  mortar gates included.
- Rules: both output pipes are drained from the moment Chromium spawns; the viewport and init
  scripts are applied before the first navigation; every launch gets its own profile directory,
  removed by `Browser::shutdown`; the Chromium child is a `tokio::process` child, with the reason
  beside the spawn.

## Related documentation

- [Editor gates](/documentation/runbooks/editor_gates.md) — the required Chromium build and the
  wedge modes the launch settings avoid.
