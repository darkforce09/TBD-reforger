# T-173 — Phase 0 inventory (perf + port-parity)

Date: 2026-07-18 · Branch `main` after T-172 @ `e08884f4`.
Measurement env: headless chromium + SwiftShader WebGL2 (`?force=webgl&sat=preview`) via
`gate smoke perf` — software raster, so absolute FPS understates every real GPU; same-env A/B
deltas are valid, and the bench FPS-equiv ≥60 floor doubles as the potato-GPU proxy (G-B).
Operator numbers (G-A ≥1000 FPS-equiv) come from the RTX 3070 rig via the HUD `rf … (eq)` cell +
`window.__editorBench(500)` in the console.

## Build-profile A/B (P8)

Operator was on **`make leptos` = `trunk serve` (debug wasm, no wasm-opt)** for the original
"unbearable" report, then manually on `trunk serve --release` for the "big improvement,
nowhere near good enough" follow-up.

| Dist | wasm bytes | pan avg fps / p95 ms / hitches | zoom avg fps / p95 ms / hitches / longtask | note |
|------|-----------|-------------------------------|--------------------------------|----------------------------------------|
| debug (`trunk build`) | 42,386,358 | (SwiftShader raster-bound) | — | wasm 5.2× bigger; no wasm-opt |
| release baseline — clean T-172 HEAD (`e08884f4`) | 8,110,933 | 13.4 / 178.2 / 38 | 44.2 / 117.1 / 31 / **1937 ms** | the operator's release baseline |
| release + T-173 (all Phase 2–5) | 8,256,081 | 15.0 / 183.3 / 39 | 50.7 / 25.7 / 16 / **565 ms** | **zoom longtask −71 %, hitches −48 %** |

**Reading the numbers.** On the CI **SwiftShader** software rasterizer the on-screen pan/zoom fps
is *raster-bound* (both baseline and after sit at ~13–50 fps regardless of CPU work), so absolute
fps is not the operator's-GPU signal — it masks the CPU wins. The measurable CPU-side improvement
is the **zoom main-thread block: 1937 ms → 565 ms (−71 %)** and zoom hitches 31 → 16. The true
frame-cost ceiling is the bench **CPU encode** row below.

## Bench — CPU frame-encode cost (the backend-independent "1000 FPS" ceiling, G-A/G-B)

`render_bench` splits per-frame **encode** (build the command list — backend-independent) from
**submit** (GL replay + raster; on SwiftShader this is software rasterization, *not* representative
of any real GPU). fps-equiv = 1000 / encode_ms.

| Camera | encode avg ms | encode p95 ms | fps-equiv (encode) | submit avg ms (SwiftShader raster only) |
|--------|--------------|---------------|--------------------|------------------------------------------|
| town (z2)   | 0.025 | 0.045 | **39,487** | 0.05 |
| forest (z0.5) | 0.052 | 0.100 | **19,296** | 33.5 |
| mid (z−1)   | 0.075 | 0.115 | **13,400** | 122.7 |
| max (z4)    | 0.023 | 0.030 | **42,689** | 135.9 |

Every camera's CPU frame-encode is **≤ 0.075 ms → ≥ 13,000 FPS-equivalent**, clearing the ≤ 1 ms /
1000-FPS budget by **13–42×**. On a GPU that keeps up with async submit (the operator's 3070, where
submit is non-blocking) that encode cost *is* the achievable frame rate. The SwiftShader submit
column (33–136 ms on heavy scenes) is the software rasterizer and is why headless on-screen fps
looks low — it does not gate anything (`S_bench_encode_60_floor` reads the encode row).

## Churn counters (release + T-173, per scenario — Phase-3 gate proof)

| Scenario | icon uploads | poly uploads | strip uploads | building uploads | glyph recomposes | fill recomposes | dup chunk fetches | idle fetches |
|----------|-------------|--------------|---------------|------------------|------------------|-----------------|-------------------|--------------|
| pan 6 s (crosses chunk boundaries → streams) | 18 | 0 | 6 | 12 | 7 | 7 | — | — |
| zoom sweep (z −2.5→3→−2.5, 32 bands) | 96 | 5 | 36 | 64 | 32 | 32 | — | — |
| settle-thrash (12 jumps + 5 s idle) | — | — | — | — | — | — | **0** | **0** |

The pan row is **not** zero because the T-173 P1 streaming settle deliberately loads chunks
mid-drag as the camera crosses boundaries (18 icon uploads / 7 recomposes over a 6 s boundary-
crossing pan ≈ the ~4/s streaming cadence) — that is the *fix*, not waste. The zero-waste property
is the **thrash gate**: after 12 camera jumps the residency re-requests **0** chunks it already has
and fetches **0** while idle (baseline had the same synthetic 0/0 here because the recovery-loop
pathology only triggered on genuinely-empty/soft-failed chunks, which this synthetic Everon path
doesn't hit — the known-empty policy still removes that pathology by construction; see verify log).

## Gate outcomes (release path, `gate smoke perf-strict`)

- **S_dup_fetches_zero** ✓ · **S_idle_fetches_zero** ✓ · **S_bench_encode_60_floor** ✓ · settled ✓ · probe_ok ✓ · panic_free ✓ → `pass: true`.
- **G-A (operator, 3070)**: pending operator run — the live HUD now shows `rf <ms> (<eq> FPS)` + `window.__editorBench(500)`; the harness proves ≥13k FPS-equiv CPU-encode headroom.

## Root-cause suspects (verified file:line — fixes land in Phases 2–5)

| P | Site | Mechanism |
|---|------|-----------|
| P1 | `world_assets/mod.rs:160-186` + `mission_editor.rs:684-692` | No mid-gesture settle: pin/draw set frozen for the whole drag; settle burst at pointerup+120 ms |
| P1 | `eden_chrome.rs:54-55` (`DOCK_L/DOCK_R` `backdrop-blur-xl`) | Compositor re-blurs two large panels over the animating canvas every frame |
| P1 | `mission_editor.rs` pointermove (CUR DEM sample per event) | Unthrottled per-event work ahead of the pan branch |
| P2 | `world_host.rs:194-219` `push_roads` | Recomposes all 888 segments + re-uploads 2 lanes every 0.5-zoom band; mesh actually varies only by 3 class-visibility signatures (`polyline_strip.rs:373-381` gates at −6.0/−2.0/4.0) |
| P2 | `world_host.rs:221-252` `push_landcover` | No memo: full 36-hull compose + upload on every `run_viewport` pass (×6/settle) |
| P2 | `residency.rs:1014-1050` `refresh_draw_set_and_glyphs` | Full glyph/strip/density re-pack on every `set_viewport`, no compose key |
| P2/P3 | `world_host.rs:317-370` `push_to_engine` | Full buffer clones + fresh GPU buffers ×7–12 per settle, no revision gating |
| P3 | `world_host.rs:145-161` empty-stub recovery | Invalidates `Some(0)` (legit-empty) and `None` (in-flight!) draw ids every settle → dup fetches forever |
| P3 | `residency.rs:600-609` `ingest_chunk_gz` | Parse-shape mismatch → `Ok(0)`, indistinguishable from real-empty (forces the host paranoia loop) |
| P4 | `missions.rs:183-185` + per-card badge `:506-509` | `backdrop-blur-xl` **on the scrollport** + N per-card blur regions |
| P5 | `ui.rs:187-192` + `missions.rs:553-625` | Two stacked backdrop-filters during the 300 ms translate + dossier fetch/mount mid-animation |
| P6 | `eden_chrome.rs:1448-1450` | Placeholder copy; engine setters exist (`set_grid` `engine.rs:2578`, `set_lane_opacity:4117` unused) |
| P7 | `eden_chrome.rs:636-648` | Per-row `border-l` guide spans broken by `py-1` padding; white/10 contrast |
| P8 | `Makefile:36` | `make leptos` = debug `trunk serve`; gates/release use `--release` + `data-wasm-opt=z` |
| P9 | `world_host.rs:317-370` | Fence/pier/rail strip lane never bridged (engine `upload_world_fence_strips:2718` + residency getters exist; zero app call sites) |
| P10 | `shader.wgsl:180` `vs_icon` | Icon lane samples atlas V-flipped (only textured lane missing the `1.0-unit.y` correction; basemap `:57` and text `:231` have it). Badge yaw is 0.0 (`residency.rs:1197-1205`); slot atlas ring+disc symmetric → flip-safe |

## Port-parity sweep (H-rows — every row is implemented in T-173)

Method: every `RenderEngine` `pub fn upload_*`/`set_*`/`ensure_*`/`clear_*` (30 fns) + every
`WorldResidency` pub fn grepped against `apps/website/frontend/src/**` call sites.

| H | Gap | Evidence | Fix phase |
|---|-----|----------|-----------|
| H1 | **Density heatmap lane never uploaded** — residency computes the R32 grid + `heatmap_trees` state (mirrored to `__mapAssets`), engine `upload_density_grid:3173` has zero app callers → zoomed-out tree density rung renders nothing | sweep: `upload_density_grid` 0 hits; `density_grid_r32_bytes`/`density_grid_dims` UNUSED | Phase 3 (with revision-gated push) |
| H2 | **Slot cluster markers never fed** — engine `on_camera_changed` hides the slot lane when `cluster_mode` (>500 slots, zoom ≤ −4, `slots_gpu.rs:44`) and waits for `set_cluster_markers` ("re-fed by TS… not ported this slice", `engine.rs:3419-3421`) → blank map in cluster band | sweep: `set_cluster_markers` 0 hits | Phase 4 (core grid-bin clusterer + host feed) |
| H3 | **Ingest budget APIs unused** — host `drain` uses flat `MAX_PER_SETTLE=24`, never `begin_ingest_frame_at`/`end_apply_frame`/`ingest_budget_exhausted_at` → single-frame ingest spikes unbounded by APPLY_BUDGET_MS | sweep: UNUSED rows | Phase 3 (streaming settle) |
| H4 | Fence/pier/rail strips (= P9) | above | Phase 5 |
| H5 | Town labels / road names / height (peak) labels — engine `upload_town_labels:3023` / `upload_road_labels:3064` / `upload_text_labels:2983` + `ensure_text_atlas:2874` zero app callers; core `locations.rs`/`road_labels.rs`/`dem/peaks.rs` complete | sweep | Phase 4b |
| H6 | Airfield lane — `set_airfield_toggle`/`set_airfield_bbox_from_runways`/`airfield_visible` UNUSED; apron mesh (`airfield.rs:107`) never built on host | sweep | Phase 4b |
| H7 | World-layer class toggles — `set_glyph_toggles`/`set_fences_toggle` UNUSED (no settings UI); `set_lane_opacity:4130` unused (hillshade slider) | sweep | Phase 4 |
| H8 | Basemap `Map` cartographic view — Leptos loads satellite only (`satellite.rs`); no `tiles/map` pyramid consumer | agent sweep §3 | Phase 4 |
| — | `pick_nearest` / `eviction_log` / `resident_chunk_ids` etc. UNUSED | introspection/gate APIs, not user-facing lanes in the React editor either — **not** H-rows | — |

## GPU memory accounting

TBD after release smoke: engine `stats()` `gpu_bytes` + `atlas_bytes` + satellite texture from the
smoke run land here (UMA iGPUs allocate from system RAM; frame-rate gates are the potato bar).
