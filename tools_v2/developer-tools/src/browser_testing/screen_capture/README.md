# Mission Creator capture drivers

The drivers behind the `capture` binary, declared in
`tools_v2/developer-tools/src/browser_testing/screen_capture.rs`: they drive the running
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) in headless Chromium to write
screenshots of its interface and of its map canvas, and crop a region out of a screenshot.

## Contents

```text
tools_v2/developer-tools/src/browser_testing/screen_capture/
├── crop.rs     `crop`: a region cut out of a screenshot, optionally upscaled by nearest neighbour
└── eval_js.rs  `shot` and `zoomsweep`, the console tap, the boot-overlay poll and the canvas read
```

## How it works

Both drivers launch Chromium with `cdp::launch_with_gpu` and `GpuBackend::Vulkan` on debug port
9222 and a 1920×1080 viewport, because the Mission Creator's WebGPU map boots only on ANGLE over
Vulkan on the real device. They need the database, the [API](/documentation_v2/glossary/a_to_f.md#api)
and the app already running.

- `shot` navigates each `<url> <waitMs>` step and sleeps for its wait, without waiting for the load
  event. It polls the boot overlay (`OVERLAY_SELECTOR`) for up to 25 s, printing the overlay text,
  the page state and the last 40 console lines, and with `--hide-overlay` removes it from the DOM.
  It writes the page with `Page.captureScreenshot`, trying `fromSurface: true` first, then
  `fromSurface: false`, then JPEG. With `--canvas` it also writes `<out>_canvas.png` from the
  canvas's `toDataURL`, because the headless compositor returns a black map; a canvas PNG under
  `CANVAS_MIN_BYTES` (20 000 bytes) is refused as a black rectangle. It exits 0 when a screenshot
  was written, 1 when none was, and 2 without a step.
- `zoomsweep` signs in through `http://localhost:8080/api/v1/auth/dev-login?role=admin`, opens
  `http://localhost:3000/missions/<id>/edit`, waits up to 60 s for the boot overlay to clear, and
  for each zoom calls `window.__editorCamSet(6400, 6400, z)` and writes the canvas to
  `<prefix>_z<z>.png` (`.` as `p`, `-` as `m`). Under headless Vulkan that call panics the map
  engine and every later read is black; the driver records this and does not work around it. It
  exits 0 when at least one zoom was written.
- `crop` cuts `w×h` at `(x, y)` with the `image` crate, upscales by an integer `scale`, and warns
  when the result passes 190 000 pixels.

## Boundaries

- Depends on: the parent's `Step`, `ShotOptions`, `CAPTURE_VIEWPORT`, `CAPTURE_DEBUG_PORT`,
  `OVERLAY_SELECTOR` and `CANVAS_MIN_BYTES`; `tools_v2/developer-tools/src/browser_testing/cdp.rs`;
  the `image` and `base64` crates; a running stack from `cargo xtask db up`,
  `cargo xtask mk rust-api` and `cargo xtask mk leptos` or `cargo xtask mk leptos-debug`.
- Used by: `screen_capture.rs`, which re-exports `shot`, `zoomsweep` and `crop`; the `capture`
  command line in `tools_v2/developer-tools/src/browser_testing/capture_cli.rs`; people.
- Rules: the capture runs on ANGLE over Vulkan, never SwiftShader; the map is read from the canvas,
  never from the compositor; the capture debug port stays apart from the gate ports (9337, 9341,
  9399).

## Related documentation

- [Editor capture](/documentation_v2/runbooks/editor_capture.md) — the stack, the GPU modes and the
  camera caveat.
