# T-151.1 verify log — W1 wgpu basemap lane

Worktree `tbd-reforger-wgpu-spike/` @ baseline `376a4cab` (tag `T-151.1-docs`, a descendant of
`f019512d`/tag `T-151.0`). Every parity claim carries its **verification class** (program
§philosophy): **R** byte/bit-exact · **T** ≤1 ULP/gray transcendental · **S** structural set
equality · **GPU‑R** margin-forced byte-exact pixel readback (operator-run, PASS = a byte compare)
· **OP** operator statement (perceptual) · **ADV** advisory screenshot diff.

## Automated gates — ALL EXIT 0

Run from the worktree root. Verbatim / summarized output:

```
$ cargo fmt --check
EXIT 0 ✓

$ cargo clippy --all-targets -- -D warnings
Finished `dev` profile — 0 warnings

$ cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
Finished `dev` profile — 0 warnings

$ cargo test -p map-engine-core --all-features
running 56 tests … test result: ok. 56 passed; 0 failed        (lib: incl. hillshade Class-T)
running  5 tests … test result: ok.  5 passed; 0 failed        (camera_props.rs)
running  5 tests … test result: ok.  5 passed; 0 failed        (deckgl_ortho_parity.rs — camera R/T)
running  0 tests … ok

$ cargo test -p map-engine-render
running 9 tests … test result: ok. 9 passed; 0 failed          (4 scene + 5 lanes, all Class R)

$ cargo build --workspace
Finished `dev` profile

$ make wasm
map_engine_wasm_bg.wasm = 3,723,192 bytes                       (baseline 3,658,383 → +64,809 B)

$ npm test                                                       (apps/website/frontend)
Test Files  41 passed (41)
     Tests  334 passed (334)                                     (baseline 317 → +17 W1 tests)

$ npm run build          # tsc -b && vite build
✓ built — WgpuTacticalMap in its own lazy chunk (WgpuTacticalMap-*.js, 10.7 kB)

$ npm run lint           # eslint .
0 problems

$ ! grep -l map_engine_wasm_bg dist/assets/index-*.js
PASS — the entry chunk has no raw-wasm reference (lazy isolation holds)
```

### New W1 tests (the +17), by class

| Test | Class | What it pins |
|---|---|---|
| `crates/map-engine-render/src/lanes.rs` `corner_uv_is_north_up` | R | NW unit(0,1) → texel (0,0); a Y-flip swaps NW/SW and fails |
| `lanes.rs` `pack_offset_places_north_at_top` | R | pyramid pack offsets `((tx-tx_min)*256, (ty_max-ty)*256)` |
| `lanes.rs` `world_rect_rel_is_anchor_relative` | R | `[0,0]..[12800,12800]` → `[-6400,-6400,6400,6400]` |
| `lanes.rs` `grid_lines_everon_pinned` | R | 52 vertices, BORDER@x=0, MAJOR@5000, MINOR@1000, exact `[173,198,255,α]` |
| `lanes.rs` `grid_lines_hs_palette` | R | over-hillshade boosted alphas (80/150/210) |
| `wgpu/pickBaseLevel.test.ts` (4) | R | Everon index × limit `{16384→0, 8192→1, 4096→2}` + `{256→6, 1→13}` |
| `wgpu/basemapLod.test.ts` (13) | S | ≥12 `(viewState,viewBounds,mode)` tuples → golden Lod + `tileUrl` south-first Y inversion |
| `_wasm/hillshade.parity.test.ts` (unchanged) | T | Rust `build_hillshade_image` ≤1 gray vs JS `buildHillshadeImage` |

The `basemapResolve.ts` extraction (L2) is proven byte-identical to the Deck path: the existing
`satelliteUnified.test.ts` / `tileUrl.test.ts` / `basemapView.test.ts` / `styleModes.test.ts` all
stay green, and `useTerrainBasemapLayer.ts` now imports the moved helpers verbatim.

## L1–L13 compliance

- **L1** — `PipelineKind::{TexturedQuad, Polyline}` added; kept live via `BatchPayload::kind()`
  (read in `stats()`). Draw order (fixed by `lane_order` upsert): basemap → hillshade → grid;
  calibration hidden in the editor (`hide_calibration`), retained on `/_spike/wgpu`.
- **L2** — `layers/basemapResolve.ts` extracted (`resolveBasemapMode`/`resolveUnifiedMode`/
  `viewFields`/`computeLod`/`clampInt` + consts/types), re-imported by the Deck hook; `parseTbdSat`/
  `pickBaseLevel`/`tileUrl` reused verbatim. Engine never parses TBDS bytes.
- **L3** — GPU upload in Rust: `tex_layer_begin`/`write_bitmap` (`copy_external_image_to_texture`,
  WebGPU) / `write_rgba` (`write_texture`, WebGL2 fallback keyed on `engine.backend()`)/`commit`.
  wgpu-29 exact API confirmed against the crate source; web-sys `ImageBitmap` feature added.
- **L4** — `pickBaseLevel` called verbatim (`engine.max_texture_dimension_2d` getter added). Golden
  matrix green.
- **L5** — pyramid LOD = `computeLod` verbatim; tiles packed one-atlas (pack offsets mirror
  `lanes::pack_offset`); `tileUrl` is the sole Y inversion. Class-S green; camera-move reload wired
  (debounced `onCameraMoved`).
- **L6** — hillshade reuses DemController's decoded `metersCache` + `manifest.dem` range →
  `wasm.hillshade` → `write_rgba` quad; opacity via `set_lane_opacity` (no Horn rebuild); memoized
  on `[terrain, showHillshade, demVersion]`; skipped when `!show`. Class-T harness green.
- **L7** — grid: `set_grid` → `lanes::grid_lines` (exact `useBaseMapLayer.ts` mirror). **Line width:
  `PrimitiveTopology::LineList` → device-native 1 px screen-space lines, matching Deck
  `widthUnits:'pixels' getWidth:1` — no world-meter width constant is used** (the primitive is 1 px
  by construction).
- **L8** — `WgpuTacticalMap` honors `terrain`/`showGrid`/`showHillshade`/`hillshadeOpacity`/
  `onBasemapDegraded`/`onBasemapProgress`; reads `mapStyle` via `useMapStyle`+`styleForMode`+
  `basemapViewForStyle`; paper tint = `set_clear_color(PAPER_TINT)` on map style (else dark field).
- **L9** — `MissionCreatorPage` passes the SAME basemap props to both mounts (shared `basemapProps`);
  interaction/`onReady`/`onCursorMove` remain Deck-only (no-ops until W7).
- **L10** — `readback_rgba(x,y)` (L10-verbatim signature) + `texture_self_check()` (synthetic
  byte-exact orientation/upload/opacity proof) added; both `probe.rs`-style offscreen readbacks.
- **L11** — unified failure → `onProgress(null)` + forced pyramid re-resolve; `onDegraded(view)` on
  mode `none`.
- **L12** — `stats()` gains `basemap_mode`/`basemap_tiles`/`basemap_bytes` **appended after** the 9
  T-151.0 keys (order/names unchanged). On the spike (batches = stress + calibration) `chunks`/
  `gpu_bytes` are byte-identical (derivation rewritten to filter `LaneRole::Stress`; proven equal by
  the scene.rs Class-R tests).
- **L13** — commit prefix `T-151.1:`, tag `T-151.1`, this log.

## GPU gates — EXECUTED in headless (SwiftShader WebGL2 + lavapipe WebGPU)

Driven via CDP against the Vite dev server + the merged wasm module (chromium
`--use-angle=swiftshader --enable-unsafe-webgpu`, software llvmpipe/lavapipe). Both backends run.
The byte-exact, regression, and integration gates below are **executed PASS** (not operator-pending);
only the perceptual look + the asset-heavy real-satellite corner remain for the operator.

- **`texture_self_check()`** — GPU‑R **EXECUTED PASS** (WebGL2). `pass:true`; probes
  NW(100,100)=`[255,0,0,255]` (**red — north-up kill-shot**, not blue=Y-flip / not green=X-flip),
  NE(700,100)=`[0,255,0,255]`, SW(100,500)=`[0,0,255,255]`, all `got == expect`. Proves the textured
  pipeline + north-up UV + `write_rgba` upload + opacity tint + offscreen readback.
- **T-151.0 `self_check()` regression** — GPU‑R **EXECUTED PASS** (WebGL2). All **7** calibration
  probes byte-exact, incl. clear=`[51,68,85,255]` and the R north-up kill-shot. **The render-loop
  refactor + the new PipelineKind/BatchPayload + the added pipelines did NOT alter the T-151.0
  calibration/quad output** (the top DO-NOT risk — confirmed byte-identical on real GPU).
- **Stress accounting** — R **EXECUTED PASS** (WebGL2). `seed_stress(1_000_000)` →
  `instances:1000000, chunks:1, gpu_bytes:32000160` (= 1 000 000·32 + 64+32+64, the exact T-151.0
  formula), `staging_peak_bytes:32000000` (one chunk = residency invariant). `stats()` shows the 9
  T-151.0 keys in order + the 3 additive `basemap_*` keys.
- **Live W1 `draw_batches`** — **EXECUTED PASS** (WebGL2). `hide_calibration` + a `Textured` lane +
  a `Lines` (grid) lane + `render()` succeed; `basemap_mode:"pyramid", basemap_tiles:1,
  basemap_bytes:16, uniform_bytes_last_frame:64`, `chunks:0` (calibration hidden, no stress).
- **Hillshade END-TO-END** — R + blend **EXECUTED PASS** (WebGL2, **real 71 MB DEM**).
  `WgpuBasemapController.setHillshade` → DemController loads the real DEM (`demReady:true`) →
  `wasm.hillshade` → GPU lane. **`basemap_bytes:3,341,584` = 914²·4** — byte-exact confirmation that
  the Class-T MAX_EDGE-1024 downsample (6400/scale7 = 914) reached the GPU. `readback_rgba(32,32) =
  [113,119,126]` — a grayscale Horn value (~153) blended `0.6·src + 0.4·[51,68,85]` (predicts
  `[112,119,126]`, ±1 unorm8), confirming the blend equation + relief rendering.
- **WebGPU `copy_external_image_to_texture`** — **EXECUTED PASS** (WebGPU/lavapipe). `backend:webgpu`;
  `tex_layer_write_bitmap` (the WebGPU fast path WebGL2 never exercises) + commit + `render()` succeed
  (`basemap_bytes:64` = 4²·4). Closes the L3 fast-path gap — **both upload backends proven**.

### Remaining operator gates (perceptual / asset-heavy) — `make web` → `?engine=wgpu`

- **S1** — OP. Satellite style: HUD `basemap: unified`; the 153 MB TBDS bundle loads (progress 0→1),
  Everon imagery visible, pan/zoom smooth. (Upload path already byte-exact-proven above; this is the
  visual/perf confirmation.)
- **S2** — OP. Map style: pyramid tiles upright + refine on zoom. **Note:** the `tiles/map/**` pyramid
  is local-gitignored (not in the repo) — needs `make map-cartographic-everon` locally first.
- **S3** — OP. Hillshade toggle + slider 0/40/100 %: relief visible, gone at 0 %; Deck path
  (`?engine=` off) unchanged. (End-to-end render already executed above.)
- **S4** — ADV. Dual-mount screenshot diff @ 3 pinned cameras (±3/channel vs Deck) — record max delta.
  Only inherently renderer-specific item = thin-line grid rasterization (geometry+color are R-proven).
- **S5** — GPU‑R (real-satellite corner). `set_view(corner, zoom=6)` → `readback_rgba` at `worldBounds`
  NW/NE/SW vs `getImageData` of the decoded base-level corner block (α=255 ⇒ pure copy; opacity 1 ⇒
  blend = `src`). The upload byte-exactness is already proven by `texture_self_check` + the hillshade
  end-to-end; this confirms it specifically for the TBDS-decoded satellite on real hardware.

## Summary

Automated verify: **PASS** (11/11 gates exit 0; 334 vitest, 75 cargo tests, 0 clippy/lint warnings,
lazy-chunk isolation holds). Merged wasm 3,723,192 B (+64,809 vs T-151.0).

GPU verify: **EXECUTED PASS** in headless (SwiftShader WebGL2 + lavapipe WebGPU) — `texture_self_check`
byte-exact (north-up), the T-151.0 `self_check` calibration regression byte-identical (all 7 probes),
stress accounting byte-identical (`gpu_bytes 32000160`), live `draw_batches`, a **real-DEM hillshade
end-to-end** (byte-exact `basemap_bytes 3341584` = 914²·4 + blend-consistent readback), and the
WebGPU `copy_external_image_to_texture` fast path. Remaining operator gates are perceptual
(S1/S3/S4) or asset-gated (S2 local tiles, S5 real-satellite corner). **Ready for Cursor doc sync.**
