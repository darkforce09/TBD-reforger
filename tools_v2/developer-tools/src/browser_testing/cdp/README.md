# Chromium launch and page setup

The launch half of the DevTools protocol client in
`tools_v2/developer-tools/src/browser_testing/cdp.rs`: finding a Chromium build, starting it
headless with the right GPU flags, draining its output, opening a page, and the small polling
helpers every gate uses.

## Contents

```text
tools_v2/developer-tools/src/browser_testing/cdp/
└── sleep_ms.rs  `find_chromium`, `launch`, `launch_with_gpu`, `new_page`, `wait_http` and the pipe drain
```

## How it works

`find_chromium` takes `CHROME_HEADLESS_SHELL` when it names an existing file, and otherwise scans
`~/.cache/ms-playwright`, newest first, preferring a full `chromium-*/chrome-linux64/chrome` build
over `chromium_headless_shell-*`, because the headless shell aborts on per-character font fallback.
`launch` is `launch_with_gpu` with `GpuBackend::Swiftshader`, the software WebGL2 path every gate
uses; the `capture` binary passes `GpuBackend::Vulkan` instead. Each launch:

- adds `--headless=new` unless the binary is the headless shell, then `--no-sandbox`,
  `--remote-debugging-port`, a fresh profile directory `tbd-cdp-<pid>-<port>` under the temporary
  directory, the backend's GPU flags, `--enable-unsafe-webgpu`, `--hide-scrollbars` and
  `--force-device-scale-factor=1`;
- sets `XDG_CACHE_HOME` on the child to the gate's own font cache from
  `tools_v2/developer-tools/src/browser_testing/diagnostics/`;
- starts Chromium in its own process group, so `Browser::shutdown` can signal the whole tree;
- drains stdout and stderr from spawn (`drain_pipe`), keeping the last 200 lines for
  `Browser::recent_output`, because an undrained 64 KiB pipe blocks whichever Chromium thread
  writes next;
- polls `/json/version` up to 80 times, 125 ms apart.

`new_page` opens a target through `/json/new`, connects its WebSocket, enables the page and
runtime domains, applies the 1440×900 viewport, registers the init scripts with
`Page.addScriptToEvaluateOnNewDocument`, and only then navigates when given a URL. `wait_http`
polls a URL until it answers 2xx or 404, and `merge` folds extra fields into a mouse or key event.

## Boundaries

- Depends on: the parent's `Browser`, `Page`, `GpuBackend` and `VIEWPORT`;
  `gate_font_cache_dir` in `tools_v2/developer-tools/src/browser_testing/diagnostics/`; `tokio`,
  `reqwest`, `tokio-tungstenite` and `libc`.
- Used by: `cdp.rs`, which re-exports every public function; through it, every gate and the
  capture harness in `tools_v2/developer-tools/src/browser_testing/`.
- Rules: both output pipes are drained from the moment Chromium spawns; the viewport and init
  scripts are applied before the first navigation; every launch gets its own profile directory,
  removed by `Browser::shutdown`.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the required Chromium build and the
  wedge modes the launch settings avoid.
