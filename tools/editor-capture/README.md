# editor-capture — screenshot the Mission Creator headless

Drives the live editor over CDP and captures both the DOM chrome and the wgpu map. Written
2026-08-01 for the editor UI/UX program; see
[`.ai/artifacts/editor_ui_program_plan.md`](../../.ai/artifacts/editor_ui_program_plan.md).

Ported to Rust at T-661 (the `capture` subcommand on the `tbd-tools` crate — same CDP plumbing as
the gate harness `smokes`); the Node/shell versions were removed to clear the `no-node`/`no-shell`
language gates. Chrome launch, the ANGLE/Vulkan flags, the KB-002 font-cache workaround and the
teardown are all inside the Rust binary now (via `cdp::launch_with_gpu(_, GpuBackend::Vulkan, _)`),
so there is no wrapper script to run.

```bash
# stack must be up: cargo xtask db up && cargo xtask mk rust-api && cargo xtask mk leptos-debug
distrobox-host-exec sh -c 'cd /path/to/TBD-Reforger && \
  CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target \
  cargo run -q -p tbd-tools --bin capture -- shot /tmp/out.png \
    "http://localhost:8080/api/v1/auth/dev-login?role=admin" 6000 \
    "http://localhost:3000/missions/<mission-id>/edit" 25000 \
    --canvas'
```

Writes `/tmp/out.png` (chrome, via CDP) and `/tmp/out_canvas.png` (the map, via `toDataURL`).
`--canvas` is the old `CANVAS_CAPTURE=1` (**required to see the map**); `--hide-overlay` is the old
`FORCE_HIDE_OVERLAY=1`. GPU mode is always ANGLE/Vulkan on the real device now (the only backend the
live wgpu engine boots on — see §2); the old `GPU_MODE=egl`/`x11` fallbacks were removed with the
shell wrapper.

## Three non-obvious things this encodes

Each cost real time to find. Change them at your peril.

**1. `XDG_CACHE_HOME` must point somewhere writable.** This host's default fontconfig cache is
read-only (ostree), so chrome finds **zero** fonts and the renderer aborts on first text layout:

```
ERROR:ui/gfx/platform_font_skia.cc:258] Could not find any font: , sans
… TextRunHarfBuzz error … glyph_count: 0
[end of stack trace]   ← renderer core-dumped
```

This is KB-002 in [`EDITOR_GATE_RUNBOOK.md`](../../docs/website/EDITOR_GATE_RUNBOOK.md); the gate
harness solves it the same way in `tbd-tools` `doctor::gate_font_cache_dir()`.

**2. `--use-angle=vulkan`, never swiftshader and never `gl`.** All three were tried:

| Mode | Result |
|---|---|
| `--use-angle=swiftshader` | `wgpu webgpu.rs:2331: createBuffer failed, size (32) too large` → wasm abort → editor stuck on the boot overlay forever |
| `--use-angle=gl` | `RenderEngine::create: webgl2 not available or canvas already in use` — engine never starts |
| **`--use-angle=vulkan`** | **Boots. Satellite basemap up, 12800² with 14 mips, `maxTextureDimension2D = 16384`.** |

Note the swiftshader path is what surfaced the boot-overlay defect (T-631 draft) — the engine dies
and the bar sits at `50% · 71.9 MB / 71.9 MB` with no failure state.

**3. The map must be read off the canvas, not the compositor.** Headless chrome logs
`Failed to initialize vulkan surface`, and `Page.captureScreenshot` — with **either** `fromSurface`
value — returns a **black map over correct DOM chrome**. That is indistinguishable from a dead
engine and is the single most misleading failure here. `canvas.toDataURL()` bypasses the
compositor and returns the real pixels.

The byte count is the tell: **~45 KB means you captured a black rectangle, ~3.7 MB means you
captured the map.** `capture shot` checks this (`CANVAS_MIN_BYTES`) and refuses to write a blank
canvas.

## Subcommands

All three are `cargo run -q -p tbd-tools --bin capture -- <sub> …` (via `distrobox-host-exec` — bare
`cargo` fails on GLIBC in the container). Source: `tools/tbd-tools/src/capture.rs`.

| Subcommand | Was | What it does |
|---|---|---|
| `shot <out.png> <url> <waitMs> [url waitMs …]` | `run_shot_gpu.sh` + `cdp2.mjs` | Launches chrome (ANGLE/Vulkan), waits for CDP, navigates the steps, polls the boot overlay out, dumps console + page diagnostics, captures chrome (`Page.captureScreenshot`) and — with `--canvas` — the map (`toDataURL`), then tears chrome down. Flags: `--canvas`, `--hide-overlay`. |
| `zoomsweep <prefix> <mission-id> <z,z,…>` | `zoomsweep.mjs` | Boots the editor, then for each zoom calls `window.__editorCamSet(6400,6400,z)`, settles, reads the wgpu canvas → `<prefix>_z<z>.png`. **See the `__editorCamSet` caveat below — it panics the engine headless.** |
| `crop <img> <x> <y> <w> <h> [scale] [out]` | `crop.sh` | Crops a region (nearest-neighbour upscale by `scale`) so it can be Read at full detail. The Read tool downscales anything over ~190,000 px, which makes small UI text unreadable — keep `W × H × SCALE²` under that. Ported to the `image` crate (no ffmpeg/python). |

## Caveats

`cargo xtask mk leptos-debug` FPS is **not** representative — the HUD read 8–57 FPS during these captures.
Judge layout, spacing, flow and copy on debug; switch to `cargo xtask mk leptos` before judging map
performance.

**`window.__editorCamSet(...)` panics the render engine under headless Vulkan.** The first call dies
at `wgpu-29.0.4/src/backend/webgpu.rs`, poisons the `RefCell` in `mission_editor.rs`'s `cam_set`,
and every subsequent `__editorCam()` returns `undefined` while every canvas read returns a ~44 KB
**black rectangle** instead of the ~3.7 MB map. This is a **headless artifact of the vulkan surface,
confirmed fine in a real browser (147 FPS)** — NOT an engine bug, and no ticket is filed against it.
`capture zoomsweep` issues the call verbatim and records this in a code comment; a black canvas from
it is the artifact. See [`.ai/artifacts/parity/camset_panic_finding.md`](../../.ai/artifacts/parity/camset_panic_finding.md).
For headless zoom that must actually move the map, drive `mouseWheel` events instead.
