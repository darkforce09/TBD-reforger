**Status:** live

# Editor capture

Screenshots a running [Mission Creator](/documentation_v2/glossary.md#mission-creator) in headless
Chromium on the real GPU: the interface through the DevTools protocol and the map straight off its
canvas. Run it to judge layout, copy and the map by eye, or to hand an agent an image of the live
editor. A capture takes about half a minute once the stack is up. The drivers are the `capture`
binary of `tools_v2/developer-tools`, described in the
[capture drivers README](/tools_v2/developer-tools/src/browser_testing/screen_capture/README.md).

## Prerequisites

- A GPU with a Vulkan driver on the machine that runs the capture: the map boots only on ANGLE
  over Vulkan (environment rule 2 below).
- The full Chromium build the [editor gates](/documentation_v2/runbooks/editor_gates.md) use,
  found the same way (`CHROME_HEADLESS_SHELL` or the Playwright cache). Check:
  `cargo run -q -p developer-tools --bin gate -- doctor` prints a `✓ chromium` line.
- The database, the [API](/documentation_v2/glossary.md#api) on port 8080 and the app on port 3000 running, as in
  [Local development](/documentation_v2/runbooks/local_development.md): `cargo xtask db up`,
  `cargo xtask mk rust-api`, then `cargo xtask mk leptos` (release) or `cargo xtask mk leptos-debug`
  (`trunk serve` without `--release`). Check: `http://localhost:3000/` answers.
- `APP_ENV=development` in `apps/website/api_v2/.env`, for the
  [dev login](/documentation_v2/glossary.md#dev-login) the capture signs in with.
- The id of a [mission](/documentation_v2/glossary.md#mission) to open, from the mission library.

## Steps

Run every command from the repository root, on the machine that owns the GPU; the binary is built
with the host's toolchain.

1. Sign in with the dev login, open the Mission Creator on a mission, and capture the interface
   and the map canvas. Each URL is followed by the milliseconds to wait after navigating to it.

   ```bash
   cargo run -q -p developer-tools --bin capture -- shot /tmp/editor.png "http://localhost:8080/api/v1/auth/dev-login?role=admin" 6000 "http://localhost:3000/missions/<mission-id>/edit" 25000 --canvas
   ```

   Expected on stderr: `→ <url> (wait <ms>ms)` for each step, the boot-overlay poll ending
   `overlay cleared after <n>s`, the page state and the last 40 console lines, then
   `OK via <mode> → /tmp/editor.png`, `canvas toDataURL → <bytes> bytes` and
   `wrote /tmp/editor_canvas.png`; exit 0. `--hide-overlay` removes a boot overlay that never
   clears from the DOM before the shot.

2. Cut a region out of a screenshot for close reading, here 400 × 300 pixels at the top left,
   doubled.

   ```bash
   cargo run -q -p developer-tools --bin capture -- crop /tmp/editor.png 0 0 400 300 2 /tmp/editor_crop.png
   ```

   Expected: `/tmp/editor_crop.png  (800x600 = 480000px)` and a warning that the image passes
   190 000 pixels. An image reader that downscales anything larger makes small text unreadable,
   so keep width × height × scale² under that.

`capture zoomsweep <out-prefix> <mission-id> <zoom,zoom,...>` writes the canvas at each zoom level
through `window.__editorCamSet`; under headless Vulkan that call breaks the map (the camera caveat
below), so its images are black.

## Verify

```bash
ls -l /tmp/editor.png /tmp/editor_canvas.png
```

Expected: both files exist, and the canvas file is megabytes, not tens of kilobytes. A canvas PNG
of 20 000 bytes or less (`CANVAS_MIN_BYTES`) is a black rectangle, and `shot` refuses to write it.
A healthy Everon map is about 3.7 MB; a black canvas is about 45 KB.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `canvas looks blank (too few bytes) — not written` | the map engine did not render: a wrong GPU mode, a panicked engine, or a wait too short for the boot | check the console lines for the engine error; raise the editor step's wait |
| `overlay STILL PRESENT after 25s: …` | the editor did not finish booting; the overlay text names the stage | read the console lines; rule 2 when it sits at a fixed download percentage |
| `/tmp/editor.png` shows a black map over a correct interface | expected: the compositor screenshot never contains the map (rule 3) | read `/tmp/editor_canvas.png` instead; pass `--canvas` |
| `capture shot: need at least one <url> <waitMs> step`, exit 2 | no URL and wait pair was given | pass at least one pair |
| the shot shows the sign-in page | the dev login step failed: the API is not in development mode or not on port 8080 | check `APP_ENV=development` and the API |
| Chromium aborts on its first text layout with `Could not find any font` | its fontconfig cache is unusable (rule 1) | clear the cache, or set `TBD_GATE_FONT_CACHE` to an empty directory |

### Environment rules

Three facts about headless Chromium that the capture encodes; changing any of them breaks it.

1. **Chromium needs a usable fontconfig cache.** With a cache it cannot use, Chromium finds zero
   fonts and the renderer aborts on its first text layout, logging
   `Could not find any font: , sans` and `glyph_count: 0`. The capture launch gets the same
   gate-owned cache as the gates (`gate_font_cache_dir` in
   `tools_v2/developer-tools/src/browser_testing/diagnostics/`); wedge mode 4 of
   [Editor gates](/documentation_v2/runbooks/editor_gates.md) has the cause.
2. **`--use-angle=vulkan`, never SwiftShader and never `gl`.** The map is a WebGPU engine, and
   only ANGLE over Vulkan on the real device boots it:

   | Mode | Result |
   |---|---|
   | `--use-angle=swiftshader` | `createBuffer failed, size (32) too large`, a wasm abort, and the editor stays on the boot overlay with no failure state, its bar stopped at the download's full size |
   | `--use-angle=gl` | `RenderEngine::create: webgl2 not available or canvas already in use`; the engine never starts |
   | `--use-angle=vulkan` | boots: the satellite basemap loads at 12800² with 14 mip levels, `maxTextureDimension2D` 16384 |

   `cdp::launch_with_gpu` with `GpuBackend::Vulkan` passes
   `--use-angle=vulkan --enable-features=Vulkan --use-vulkan --ignore-gpu-blocklist`; the gates
   keep SwiftShader because they need no GPU and do not boot the WebGPU map.
3. **The map is read off the canvas, not the compositor.** Headless Chromium logs
   `Failed to initialize vulkan surface`, and `Page.captureScreenshot` returns a black map over a
   correct interface with either `fromSurface` value, which looks exactly like a dead engine.
   `canvas.toDataURL()` bypasses the compositor and returns the real pixels; `--canvas` writes
   them to `<out>_canvas.png`.

### Camera caveat

`window.__editorCamSet(x, y, zoom)` (installed by
`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs`) panics the render engine under
headless Vulkan: after the first call every `__editorCam()` returns `undefined` and every canvas
read is a black rectangle of about 44 KB. In a real browser the call works and the editor renders
at full frame rate, so this is an artifact of the headless Vulkan surface, not an engine defect.
`capture zoomsweep` makes the call as it is and does not work around it. For a headless zoom that
has to move the map, drive mouse wheel events instead. The measurement is in
[camset_panic_finding.md](/.ai/artifacts/parity/camset_panic_finding.md).

A debug build (`cargo xtask mk leptos-debug`) renders far below release frame rates; judge layout,
spacing, flow and copy on it, and switch to `cargo xtask mk leptos` before judging map
performance.

## Related

- [Mission Creator capture drivers](/tools_v2/developer-tools/src/browser_testing/screen_capture/README.md)
  — what `shot`, `zoomsweep` and `crop` do, step by step.
- [Developer tool executables](/tools_v2/developer-tools/src/bin/README.md) — the `capture`
  synopsis and exit codes.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the headless gates on SwiftShader,
  and the font-cache wedge.
- [Editor UI program plan](/.ai/artifacts/editor_ui_program_plan.md) — the Mission Creator
  interface work the capture serves.
