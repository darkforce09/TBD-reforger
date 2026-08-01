# editor-capture — screenshot the Mission Creator headless

Drives the live editor over CDP and captures both the DOM chrome and the wgpu map. Written
2026-08-01 for the editor UI/UX program; see
[`.ai/artifacts/editor_ui_program_plan.md`](../../.ai/artifacts/editor_ui_program_plan.md).

```bash
# stack must be up: make db-up && make api && make leptos-debug
distrobox-host-exec env GPU_MODE=vulkan CANVAS_CAPTURE=1 \
  bash tools/editor-capture/run_shot_gpu.sh /tmp/out.png \
  'http://localhost:8080/api/v1/auth/dev-login?role=admin' 6000 \
  'http://localhost:3000/missions/<mission-id>/edit' 25000
```

Writes `/tmp/out.png` (chrome, via CDP) and `/tmp/out_canvas.png` (the map, via `toDataURL`).

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
captured the map.** `cdp2.mjs` checks this and refuses to write a blank canvas.

## Files

| File | What it does |
|---|---|
| `run_shot_gpu.sh` | Launches chrome with the right flags, waits for CDP, runs the driver, tears chrome down. Self-contained — nothing has to survive between invocations. |
| `cdp2.mjs` | Zero-dep CDP driver (Node ≥22 has a global `WebSocket`). Navigates, polls the boot overlay out, dumps console + page diagnostics, captures. |
| `crop.sh` | Crops a region so it can be read at full detail. The Read tool downscales anything over ~190,000 px, which makes small UI text unreadable — keep `W × H × SCALE²` under that. |

## Env

| Var | Effect |
|---|---|
| `GPU_MODE` | `vulkan` (default) · `egl` · `x11` (real window on `:0`) |
| `CANVAS_CAPTURE=1` | Also write `<out>_canvas.png` via `toDataURL` — **required to see the map** |
| `FORCE_HIDE_OVERLAY=1` | Remove the boot overlay from the DOM before capturing, to see the chrome behind a stuck boot |

## Caveat

`make leptos-debug` FPS is **not** representative — the HUD read 8–57 FPS during these captures.
Judge layout, spacing, flow and copy on debug; switch to `make leptos` before judging map
performance.
