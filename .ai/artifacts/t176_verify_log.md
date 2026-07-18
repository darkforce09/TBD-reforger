# T-176 verify log — Forest fidelity + place ghost + zoom+pan stutter

Start: `main` @ `b90deac8` (T-175). Authority: `docs/platform/t176_forest_place_ghost_zoompan.md`.
Inventory: [`t176_inventory.md`](t176_inventory.md). Screens: `t176_operator_screens/01–03`.

Operator scope decision (this session): **A2 = re-bake density to 8 m now** (32 m too coarse),
re-derived from committed objects (no Workbench/staging). 4 m declined.

## Matrix

| ID | Bug | Fix | Status |
|----|-----|-----|--------|
| A1 | Forest highlight stale until zoom-in→out | `push_landcover` ordered after `set_viewport` (no one-pass lag); A2 removes the stranded wash layer entirely | ✅ code + build; operator G-A cold-open eye |
| A2 | Wash far outside trees / fills clearings | Drop forest-kind landcover wash; forest = 8 m canopy-blurred TBDD mass (iso=2) + glyphs | ✅ data-verified tight (below); operator G-A taste |
| B1 | Place ghost invisible during palette drag | `engine.rs` draw batch: bind `SlotPlacePreview` to the slot atlas | ✅ code + build; operator G-A drag |
| B2 | Zoom+pan stutter (zoom alone OK) | Defer DEM-contour + 8 m forest recompute while a pan gesture is active; full recompute on pointer-up | ✅ code + build; operator G-A `__editorBench` |

## A1 — forest cold-open stale

Root: the loose **landcover wash** (lane 1) was stranded — `push_landcover` ran *before*
`set_viewport` (world_host.rs), reading the previous pass's `deck_zoom`/tree-pack state, and the
early settle-loop break denied the corrective pass, so the wash stuck over a z≥0 view until a camera
change ("zoom all the way in then out resets it"). Fixes:
- `world_host.rs::run_viewport` — `push_landcover` now runs **after** `set_viewport` (live zoom).
- A2 removes the forest-kind wash entirely → the stranded layer no longer exists; forest is the 8 m
  TBDD mass, which renders on a stable cold open (default zoom −2.0 → fill on; final `push_composite`
  reflects the fully-resolved chunk set). No zoom ritual.

## A2 — 8 m re-bake + drop wash

**Re-bake (no Workbench):** new `world redensify` subcommand reads committed
`objects/prefabs.json.gz` (prefabId→kind) + all 625 `objects/chunks/*.json.gz` instances, accumulates
a global 8 m corner grid, **box-blurs** the tree channel into a canopy field, slices per chunk,
overwrites `objects/density/*.bin`. Output:

```
[redensify] everon: 501861 trees, 307431 rocks -> 25x25 = 625 bins, 10572500 B; cell 8 m, blur r=1 (canopy)
```

**501,861 trees** matches the census exactly → committed-chunk extraction correct. Data model:
`DENSITY_CELL_M 32→8`, `DENSITY_COLS/ROWS 17→65`; `box_blur_corners` (r=1, ~24 m window, global →
seamless per-chunk); frontend marches at `CANOPY_MASS_ISO=2` (core `DENSITY_ISO=2` unchanged, still
pinned by `density_iso_is_two`).

**Drop wash:** `push_landcover` filters `kind != "forest"`. Everon's 36 regions are all forest-kind →
landcover lane now empty (cleared, not a zero-poly upload). No density-heatmap glow reintroduced.

**Data-level tightness check** (decode 8 m tree channel, fill = corners ≥ iso=2 → marching-squares
fill fraction):

| chunk | maxTree | nonzero | filled (≥2) | reading |
|-------|---------|---------|-------------|---------|
| 10_10 | 27 | 31% | **21%** | tight canopy, holes for clearings |
| 12_12 | 54 | 62% | **59%** | dense forest, solid |
| 13_11 | 30 | 52% | **39%** | mixed, tight |
| 5_5, 20_20 (water) | 0 | 0% | **0%** | no false forest |

Forested chunks 21–59% filled (vs the old ~100% mega-hull wash); water/open = 0%. Clearings stay open
by construction (density < iso → hole). Final aesthetic tightness = operator G-A; a tweak is a
one-const change (`CANOPY_KERNEL_RADIUS_CELLS` / `CANOPY_MASS_ISO`) + `world redensify` re-run.

Golden S13 fixture regenerated at 8 m via `world gen-density-fixture` (65×65, 16916 B) — S13 PASS.
`manifest.json densityCellM 32→8`. Density bins stay plain git (10.5 MB; consistent with prior
convention — LFS migration of already-tracked `.bin`s skipped to avoid pointer churn).

## B1 — place ghost

Root: `engine.rs::draw_batches` icon-atlas bind match omitted `LaneRole::SlotPlacePreview` → the ghost
ring drew against the world glyph atlas (wrong UVs → invisible) or was skipped when that atlas is
`None` on a cold map. The pointer path was already correct (container-level `pointermove` fires
`set_place_preview`). Fix: bind `SlotPlacePreview` to `slot_base_bind` like `Slots`. Drag-move preview
(`SlotDrag`) and drop (`Slots`) already worked — this makes the mid-drag ghost use the same atlas.

## B2 — zoom+pan stutter

Root: the ~250 ms max-latency settle floor fires for the whole gesture (every pan `pointermove`
re-arms it), and every wheel tick mutates zoom, so each forced settle ran the heavy synchronous
zoom-band recompute — chiefly `dem.sync` contour marching-squares — mid-drag. Fix: a thread-local
`CAMERA_GESTURE` flag (set on pan pointer-down, cleared on pointer-up); `flush_viewport` skips
`dem.sync` **and** the (now 16× heavier) 8 m forest recompute while it's set, keeping world chunk
streaming. The pointer-up settle runs the full recompute once. Zoom-alone leaves the flag false →
unchanged (contours update per wheel-pause). `__editorBench(500)` zoom+pan before/after = operator
G-A (interactive gesture).

## Gates

| Gate | Result |
|------|--------|
| `cargo check -p website-frontend --target wasm32` | PASS |
| `cargo test -p map-engine-core --all-features` | PASS (5/5; `density_iso_is_two` intact) |
| `make schema-validate` (incl. S13 density fixture) | PASS |
| `leptos-build` (release, in leptos-gates) | PASS (optimized build, my changes) |
| `gate editor-suite` (headless smokes) | ⚠ CDP `Runtime.evaluate` timeout — documented headless software-WebGPU/lavapipe wedge (`t166-editor-smoke-webgl`), pre-existing, not T-176 code. On-GPU render = operator G-A (established T-17x division). |
| `make ci-local` | **PASS** — editorconfig, no-python, no-node, rust-ci (wasm-ci clippy `-D warnings` on map-engine-core/render + all backend tests), coding-standards, ci-local-leptos (website-frontend fmt + clippy wasm32 + tests + trunk `--release`), ci-local-schema (validate + 16 @contract citations). No errors. |

## Manual notes vs screens 01–03 (operator G-A)

- `01` loose wash / `02` island overwash → removed (forest-kind landcover dropped); zoomed-out forest
  = tight 8 m mass (21–59% forested-chunk fill, water 0%).
- `03` clearings painted → clearings now holes (density < iso); darker real-canopy shows through.
- Contours + town labels: untouched (no regress).

## Cursor doc list (Composer 2.5)

- CLAUDE.md §Status: new **T-176** shipped bullet under the T-175 line; bump latest shipped → T-176.
- Note the forest render-model change: forest highlight = **8 m TBDD canopy mass** (`DENSITY_CELL_M=8`,
  `box_blur_corners` r=1, `CANOPY_MASS_ISO`), loose 32 m landcover forest **wash removed**; `world
  redensify` / `world gen-density-fixture` re-bake path (committed-chunk, no Workbench).
- Registry `.ai/tickets/registry.json`: T-176 → `shipped` (sync).
- Links: this log + `t176_inventory.md`.
