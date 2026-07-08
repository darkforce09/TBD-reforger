# T-151 — wgpu Mission Creator engine program (W0–W9 master blueprint)

**Status:** program hub (authority for all T-151.x slices) · **Worktree:**
`tbd-reforger-wgpu-spike/` (absolute: `/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`)
— the operator's standing worktree instruction supersedes the commit-to-main convention for this
program. Integration ref `t-151-wgpu-spike` is git metadata only; agents do **not** manage
branches per slice. · **Spike shipped:** commits `152b3a12…94261dd6`
(camera parity + render spine + 20M stress + byte-exact self-check, verify log
[`t151_wgpu_spike_verify_log.md`](../../../.ai/artifacts/t151_wgpu_spike_verify_log.md)) ·
**W0 / T-151.0 shipped:** @ `f019512d` (tag **T-151.0**) — verify log
[`t151_0_verify_log.md`](../../../.ai/artifacts/t151_0_verify_log.md) · **W1 / T-151.1 shipped:**
@ `3ab81587` (tag **T-151.1**) — basemap TBDS/hillshade/grid on wgpu; verify log
[`t151_1_verify_log.md`](../../../.ai/artifacts/t151_1_verify_log.md) · **Next slice:**
**T-151.2** (W2 world parser in Rust) — `ready`; slice spec
[`t151_2_world_parser.md`](t151_2_world_parser.md).

## In one sentence

Wire the full Mission Creator tactical map — world data, spatial indexing, all fifteen render
layers, and every editor interaction — onto the T-151 wgpu/wasm engine, one gated slice at a
time, with the Deck.gl implementation kept as a live oracle until the final flip (T-151.9).

## Execution model (worktree-only)

All T-151.x slices (W0–W9) run in the **standing worktree** with **linear commits** — not via
per-slice branches and not via `./scripts/ticket run` (which would spawn a nested worktree under
`.ai/artifacts/worktrees/`).

```mermaid
flowchart LR
  operator[Operator pastes prompt]
  claude[Claude Code in spike worktree]
  commits[Linear commits T-151.0 then T-151.1]
  cursor[Cursor doc sync after each slice]
  operator --> claude --> commits --> cursor
```

Rules (binding for every T-151.x slice):

1. **CWD:** repo root = `tbd-reforger-wgpu-spike/` (absolute path above).
2. **No branch churn:** do **not** `git checkout -b`, do **not** create `ticket/T-151.x`
   branches, do **not** merge slice branches. Each slice = one (or few) commits on the
   worktree's current HEAD, tagged `T-151.x`.
3. **No `./scripts/ticket run` for T-151:** prompts are pasted manually (Cursor Mode B → Claude
   Code). The queue `branch` field (`t-151-wgpu-spike`) is sync metadata only.
4. **Do not touch `main`** from the spike worktree until the program merges back.
5. **Preflight checks worktree location**, not branch name:
   - `git rev-parse --show-toplevel` ends with `tbd-reforger-wgpu-spike`
   - `git status --porcelain` empty (or only expected files)
   - Baseline SHA from prior slice's verify log / tag
6. **Future slice specs** (T-151.1+) inherit this section verbatim; each gets its own spec
   file + handoff + one fenced prompt — same worktree, next commit.

The worktree may remain checked out to `t-151-wgpu-spike` (git requires a branch on a
worktree) — that is **not** something agents manage per slice.

## Slice prompt template (W1+)

Every slice spec ends with a `§Claude Code prompt` block. All prompts share this skeleton
(slice-specific READ/DO/VERIFY sections replace the placeholders):

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # must be empty @ baseline SHA from prior slice tag/log
  # Do NOT checkout or create branches; do NOT run ./scripts/ticket run

═══ READ (in order — spec wins on conflict) ═══
  …

═══ DO NOT ═══
  …
  - git checkout -b / create ticket/T-151.x branches
  - ./scripts/ticket run

═══ RETURN ═══
  - Commit SHA + tag T-151.n
  - .ai/artifacts/t151_{n}_verify_log.md (record git rev-parse --show-toplevel + HEAD SHA)
  - **Ready for Cursor doc sync.**
```

## Verification philosophy (binding for every slice)

Every slice inherits the house parity discipline (`features/_wasm/parity.ts` classes):

- **Class R** — byte/bit equality (`f32BytesEqual`, integer counters, memcmp of upload bytes).
- **Class T** — transcendental, ≤ 1 ULP (`ulpDistanceF64`).
- **Class S** — structural, result-**set** equality (chunk membership, pick sets, LOD tables).
- **GPU readback probes** — byte-exact pixel assertions with **margin-forced** exactness
  (edges on integer pixel coordinates, colors with unorm8 rounding headroom ≥ 3 orders of
  magnitude over f32 error — the T-151 spike pattern).
- **Measured numbers** — `RenderEngine.stats()` fields (`gpu_frame_ms` via `TIMESTAMP_QUERY`
  where offered, `uniform_bytes_last_frame`, exact instance counters) recorded in per-slice
  verify logs. fps is a readout, never an eyeball.
- The **JS/Deck implementation stays in the tree as the oracle** for every ported system until
  T-151.9 (the F1→F4 discipline that retired Yjs). Deck.gl remains a devDependency forever as
  the camera oracle.

Anything irreducibly perceptual (pan feel) is recorded as an explicit operator statement,
never claimed as verified.

## Pinned inventory (every number is an exact assertion somewhere in W1–W8)

| Quantity | Value | Source of truth |
|---|---|---|
| Prefabs | **391** | `manifest.json` `objects.prefabCount` |
| World object instances | **508,291** | `objects.instanceCount` |
| Chunk files | **275** (512 m grid, `floor(x/512)`) | `objects/chunks/`, `chunkMath.ts` |
| Road segments | **888** (6 classes incl. runway) | `roads.json.gz` |
| Forest regions | **36** | `forest-regions.json.gz` |
| TBDD density grids | **625** × 1,172 B (17×17 u16 corners, 32 m cells, 2 channels) | `objects/density/`, `tbdd.rs` |
| World glyph atlas | **28** glyphs | `glyphs/atlas/world-glyphs.json` |
| TBDS satellite | 12800² base, **14** mips, **152,713,114 B** | `manifest.json` `tiles.satellite.unified` |
| DEM | 6400² u16, **−204.78 … 375.53 m**, no axis flip | `manifest.json` `dem` |
| Zoom band | **−6 … +6**, default **−2** | `useOrthographicView.ts:12–13,33` |
| Slot pick radius / drag threshold | **4 px** / **4 px** | `slotSpatialIndex.ts:123`, `useSelectTool.ts:21` |
| Cluster gates | > **500** slots AND zoom ≤ **−4**; pick **48 px**; super-zoom `round(z+8)` clamp 0–16 | `constants.ts`, `slotClusterIndex.ts` |
| World pick radius | **12 px** | `t090_render_lod_contract.md` §N2 |
| Instance budget (legacy Deck cap, to be lifted) | **150,000** | `worldObjectsCore.ts` |
| Chunk apply budget | ≤ **4 ms**/frame | `chunkStore.ts` `APPLY_BUDGET_MS` |
| Chunk LRU | `max(64, 3 × pinned)` | `chunkStore.ts`, `worldObjectsCore.ts:658–676` |
| Spike constants | engine chunk pool 2,097,152 × 32 B = 64 MiB; scene anchor (6400, 6400); nav invariant 64 B/frame | `crates/map-engine-render/src/scene.rs` |
| Measured GPU constant | ≈ **0.69 ms per 1M instances** (32 B layout, operator hardware, `gpu_frame_ms` 13.9–14.4 @ 20M) | spike verify log |

LOD gate authority: [`t090_render_lod_contract.md`](t090_render_lod_contract.md) §N2/§N3 +
`worldmap/lodGates.ts` (`TREE_GLYPH_MIN_ZOOM=0`, `BUILDING_FOOTPRINT_MIN_ZOOM=−2.5`,
`BUILDING_BADGE_MIN_ZOOM=+1`, `VEGETATION_MIN_ZOOM=+1.5`, `PROP_MIN_ZOOM=+3`, forest fill max
+1, contour ladder 100/50/20/10 m, road class gates).

## Locked architecture decisions (D1–D4)

- **D1 — One wasm module, one memory.** `map-engine-render` becomes a dependency of
  `map-engine-wasm` (bundler target, existing `make wasm` output). `MissionDoc.slot_xy_ptr`,
  future `WorldStore` chunk columns, and the `RenderEngine` then share one linear memory —
  doc→GPU and world→GPU uploads are `queue.write_buffer` over in-memory slices, zero JS
  copies. The `--target web` spike pkg is retired (T-151.0). Editor-route code-splitting keeps
  the merged wasm out of the entry chunk (machine-gated).
- **D2 — The world-object worker is retired, not ported.** Chunks are fetched + gunzipped by a
  thin JS async loader (`DecompressionStream`), parsed **once** in Rust under the ≤ 4 ms/frame
  amortized budget, uploaded **once** to per-chunk GPU buffers, CPU copy reduced to pick
  columns. There is no per-frame JS consumer left, so the SharedArrayBuffer question from the
  zero-copy kickoff is moot. Tripwire: if parse hitching is measured above budget in `stats()`,
  the worker path is the documented fallback (D2 is reversible; the Rust parser is
  thread-agnostic).
- **D3 — Dual-mount migration, Deck as oracle.** `MissionCreatorPage` renders the Deck
  `TacticalMap` or the new `WgpuTacticalMap` behind `VITE_MC_ENGINE=wgpu` + a `?engine=wgpu`
  runtime override, both implementing the **same props contract** (the `TacticalMapProps` of
  `tactical-map/TacticalMap.tsx`: `onEntitiesMove`, `onEntityActivate`, `onAssetDrop`,
  `onClusterDrill`, terrain, `TacticalMapApi.flyTo`, cursor channel). Deck code is deleted only
  at T-151.9 after the interaction-parity suite passes.
- **D4 — Current asset wire only.** All ingestion reads the existing formats (JSON-gz chunks,
  TBDD, TBDS, roads/regions JSON-gz, DEM PNG). A Rust-native binary chunk wire requires a human
  Workbench re-export (executor gate) and is a named deferred slice, not part of W0–W9.

## Slice map (registry `T-151.0` … `T-151.9`; W10 = separate tickets unlocked at the end)

### T-151.0 (W0) — wasm packaging merge + engine batch list + editor dual mount

**Shipped:** @ `f019512d` (tag **T-151.0**, 2026-07-08) — verify log
[`t151_0_verify_log.md`](../../../.ai/artifacts/t151_0_verify_log.md). Merged
`map_engine_wasm_bg.wasm` = **3,658,383 B** (baseline 931,424; +2.6 MB engine). L3 start-fn
collision not needed. `WgpuTacticalMap` lazy-loaded (Vite dep-scan + raw `_bg.wasm` import).
Automated gates exit 0; browser GPU manuals S1–S3 operator-pending.

Merge per D1; refactor engine internals from hardcoded draws to an ordered `Vec<Batch>`
(pipeline kind + buffers + range + visibility — behavior identical this slice); mount
`WgpuTacticalMap` in the editor shell behind the D3 flag showing the calibration scene.
Slice spec: [`t151_0_wasm_merge_dual_mount.md`](t151_0_wasm_merge_dual_mount.md).
**Gates:** all shipped spike gates re-run green on the merged pkg (self-check byte-exact both
backends, 20M stress re-recorded); vitest baseline (317) + moved tests green; entry-chunk
isolation `! grep -l map_engine_wasm_bg dist/assets/index-*.js`; shared-memory numeric proof
(seeded `MissionDoc` slot view over the same `memory.buffer` the engine uses: 2,000 floats all
finite ∧ ∈ [0, 12800] → displayed PASS); merged wasm byte size recorded.

### T-151.1 (W1) — basemap lane: TBDS satellite, hillshade, grid, pyramid fallback

**Shipped:** @ `3ab81587` (tag **T-151.1**, 2026-07-08) — verify log
[`t151_1_verify_log.md`](../../../.ai/artifacts/t151_1_verify_log.md). Merged
`map_engine_wasm_bg.wasm` = **3,723,192 B** (+64,809). Vitest **334** (+17). GPU gates executed
byte-exact via headless CDP (`texture_self_check`, T-151.0 self_check regression, hillshade
end-to-end on real DEM). S1/S2/S4/S5 perceptual or asset-gated items documented in verify log.

Slice spec: [`t151_1_basemap_lane.md`](t151_1_basemap_lane.md) (authority for L1–L13 + prompt).

JS keeps the proven TBDS container parse (`satelliteUnified.ts`: magic `TBDS`, JSON index,
VP8L blocks → `ImageBitmap`); engine gains basemap texture creation + per-mip-tile upload
(`copyExternalImageToTexture`; WebGL2 fallback path: RGBA decode + `write_texture`) and draws
one world-bounds textured quad, trilinear; `pickBaseLevel` honors `maxTextureDimension2D`
exactly. Hillshade: existing Rust `dem_decode_png_to_meters` + `build_hillshade_image` →
second quad with the 0–100 % @ 0.1 % opacity uniform (T-090.1.2.6 contract). Grid: procedural
1 km lines (~40) + border per `useBaseMapLayer.ts`. Map-view **pyramid fallback** (≤64 tile
quads, south-first Y per `tileUrl.ts`) + single-ortho + `none` degrade chain preserved.
**Gates:** texture corner probes — readback at projected `worldBounds` NW/NE/SW equals the
source mip corner texels byte-exact (north-up proof for textures); mip-selection golden test
(canvas size + zoom → expected level, exact integer); hillshade Class T ≤ 1 gray level
(existing harness); dual-mount screenshot diff at 3 pinned camera states (advisory ±3/channel).

### T-151.2 (W2) — world parser in Rust (`world/` module; kickoff Piece 1)

Slice spec: [`t151_2_world_parser.md`](t151_2_world_parser.md) (authority for L1–L13 + prompt).

New `crates/map-engine-core/src/world/`: `WorldStore` (manifest, prefab table, chunk registry)
+ `WorldChunk` SoA with columns exactly as `worldObjectsCore.ts:571–617` produces them
(`positions: Vec<f32>` xy-pairs, `prefab_idx: Vec<u16>`, `rotations: Vec<f32>`,
`z: Vec<f32>`, `cls_codes: Vec<u8>` with 255 = skip, per-class row lists) — same `as f32`
operation order (Class R). Parsers: chunk instance JSON (`[prefabId, x, y, z, rotationDeg]`
rows), `prefabs.json.gz` (halfExtents, render class, iconKey, baseSizePx, defaultColor,
importanceZoom), `roads.json.gz` + a Class-T port of `extractRoadCenterline`,
`forest-regions.json.gz`, TBDD (`decode_tbdd` exists). Building OBB corner trig = Class T
≤ 1 ULP. Wasm surface: `load_manifest`, `parse_chunk(cx, cy, bytes)`, per-chunk `*_ptr/len`
views, exact counters.
**Gates (JS parser is the oracle):** `features/_wasm/world.parity.test.ts` — for **all 275**
real chunk files: SoA columns byte-equal to `worldObjectsCore` output (Class R); per-class row
sets equal (Class S); totals assert exactly **391 / 508,291 / 275 / 888 / 36 / 625**; OBB
corners + road centerline vertices ≤ 1 ULP.

### T-151.3 (W3) — chunk residency + world spatial index + first world instances

Residency in Rust: viewport → chunk-id set (512 m grid math mirror of `chunkMath.ts` + 5 %
preload margin + oversized ring), LRU `max(64, 3 × pinned)` with pinned immunity, ≤ 4 ms/frame
amortized parse/upload budget, 12-way fetch concurrency preserved in the JS loader. On
residency: per-chunk, per-render-class GPU instance buffers; CPU copy retained only for pick
columns. Chunk-keyed `PointIndex` (class-filtered `pick_nearest`/`pick_rect`) replaces the
worker rbush. First visuals: buildings as rotated-OBB instances + thin outline (instance
layout gains rotation — the step toward the ≤ 20 B production layout).
**Gates:** scripted deterministic pan/zoom path → **identical chunk-id sets** to `chunkStore`
at every step and identical LRU eviction order (Class S); per-chunk instance counts exact;
apply budget measured ≤ 4 ms in `stats()`; 10k scripted picks — result sets equal to the
worker rbush (Class S); building readback probe at a pinned camera (OBB center pixel = fill
color, byte-exact).

### T-151.4 (W4) — vector layers: sea, contours, roads, forest, landcover, marquee

Rust geometry (already produced: `sea_band`, `contours`, `forest_mass`, TBDD) feeds engine
meshes directly. New pipelines: **polygon** (triangulated fills, per-vertex color — sea
hypsometric bands, forest fills, 36 landcover hulls, marquee rect) and **polyline**
(meter-width strips with casing + dash — 6 road classes, contour segments, forest outlines).
Contour interval ladder (100/50/20/10 m) and fade ladders (`sea_fill_alpha`,
`forest_fill_alpha`) verbatim from the existing Rust functions.
**Gates:** triangulation **area conservation** — Σ triangle areas == ring polygon area within
a stated ULP-scaled tolerance, per polygon (the triangulator's forced correctness check);
polyline width exactness at segment midpoints (projected width == `widthM · 2^zoom` px ±1e-6);
geometry buffers byte-equal to the existing Class R harness outputs; readback probes (sea band
color at a known ≤ 0 m texel; road centerline pixel at a pinned camera); per-layer isolated
dual-mount screenshot diffs.

### T-151.5 (W5) — glyph atlas + LOD gates: trees, props, badges, slot ring, cluster discs

Atlas `world-glyphs.webp` + JSON (28 glyphs) uploaded once; **production icon instance layout
pinned** (≤ 20 B: pos 2×f32 = 8, size 4, rotation snorm16 = 2, glyph u16 = 2, tint u32 = 4 —
the §20M budget from the spike plan); per-instance UV via a 28-entry uniform table
(WebGL2-safe, asserted at init). Slot ring, cluster disc, and 3 building badges join the
atlas. Size math Class R: `baseSizePx × treeSizeMultiplier(heightM) / 2^REF_ZOOM` (REF_ZOOM=3),
`sizeMinPixels`, badge 10 px min 8. LOD gate table ported to Rust consts with per-class user
toggles (`worldLayerPrefs` bridge).
**Gates:** exhaustive LOD equality scan — Rust `class_visible(class, zoom)` ==
`lodGates.ts` for every class × zoom ∈ {−6.0, −5.9, …, +6.0} (121 zooms × all classes,
Class R); glyph UV rect golden test (atlas JSON → UV exact); size math Class R across the 51
tree types; tree glyph readback (nonzero alpha at projected center + tint class match);
instance accounting exact in `stats()`.

### T-151.6 (W6) — mission entities zero-copy: slots, selection, drag overlay, clusters

`MissionDoc.refresh()` → engine reads `slot_xy_ptr/slot_len` in-memory (D1) into a slot
instance buffer with **dirty-range uploads** (O(edited) per `_patch*` class: add / bulk-add /
remove / move; full re-upload only on `_applySnapshot` = undo/boot). Selection tint =
per-instance flag column (O(selection)). Drag overlay = T-061 dual-layer contract: base
excludes dragged ids, small overlay buffer + a **delta uniform** (no per-frame re-upload),
commit patches then restores. Cluster discs from the existing Rust `ClusterIndex` under the
exact gates (> 500 slots, zoom ≤ −4, drill +1). `slotIconCache` slims to index maintenance.
**Gates:** rendered instance count == `slot_len` exact after scripted mutation sequences
(add / paste 10k / delete / undo / redo); selection flag population == `selection.ids.length`;
drag math — overlaid instance projects at `base + (dx,dy)` px ±1e-9 with
`uniform_bytes_last_frame` = 64 + one 16 B delta during drag; undo → buffer bytes equal
re-materialized SoA (sampled rows, Class R); criterion-6 re-run at 500k seeded slots (fps +
`gpu_frame_ms` recorded).

### T-151.7 (W7) — interaction rewire + parity suite

Every `view.makeViewport(...)` consumer swaps to the ULP-0 camera (`useSelectTool` pan/drag/
marquee/pick-radius `r_world = unproject(px+4) − unproject(px)`; cursor rAF channel + DEM z;
asset-drop unproject; dbl-click pick; basemap bounds). The gesture machine (4 px threshold,
rAF coalescing, pending-left/move/marquee/pan, Ctrl toggle, cluster drill) is **not**
redesigned — only its camera calls change. Picks stay on wasm `SlotIndex` (4 px) +
`ClusterIndex` (48 px) + world `PointIndex` (12 px). Keyboard/DnD/modal contracts unchanged
(Ctrl+C/V with the 500 select cap, Space flyTo → `set_view`, Delete, dbl-click ≤ 1 guard,
`ASSET_DND_MIME`).
**Gates:** the **interaction parity suite** — scripted synthetic pointer/keyboard sequences
(click, Ctrl toggle, marquee, drag-move 100 slots, paste-at-cursor, cluster drill, drop) run
against the Deck mount and the wgpu mount produce **identical selection sets** and
**identical doc mutations** (`encode_state` bytes equal after each script — Class R); CUR/SEL
Z equals `sampleElevation` (Class R).

### T-151.8 (W8) — culling + the density ladder productionized

CPU chunk cull on both backends (draw set = resident chunks ∩ `visible_world_rect` + margin).
L2 density overview: TBDD grids → density texture; when a class's **exact** visible-count
estimate exceeds budget, its glyph batch swaps to the heatmap quad (aggregate rung), switch
driven by exact per-chunk integer counts. WebGPU-only compute cull for boundary chunks and
≥ 1M-slot missions (`VERTEX|STORAGE` compaction + `draw_indirect`); WebGL2 keeps chunk
granularity. Damage-driven rendering (render on camera/doc/residency change; fps HUD keeps a
continuous mode).
**Gates:** draw-chunk set == reference viewport set per scripted frame (Class S); density
texel sums == exact chunk instance counts (Class R integers); compute-cull readback count ==
CPU reference count over 1k random frusta (Class R); measured band table (fps +
`gpu_frame_ms` per LOD band, full Everon dataset + 367k-slot mission) in the verify log.

### T-151.9 (W9) — flip + Deck retirement (the F4 analog)

Default `VITE_MC_ENGINE=wgpu`; after soak, delete Deck layer modules (`useIconLayer`,
`useClusterIconLayer`, `useSelectionLayer`, `useDemLayer`, `useTerrainBasemapLayer`,
`useBaseMapLayer`, `worldmap/*Layer.ts`, `useWorldMapLayers`), the world worker trio, the
Deck-side stores, and deck.gl from the runtime bundle (stays devDependency — camera oracle).
**Gates:** full editor E2E (load ~367k mission from IDB + server, edit, Save Version 201,
Export download, conflict path); vitest green with oracles retargeted; bundle delta recorded;
before/after fps table; Cursor doc-sync pass (registry ship states, CLAUDE.md §Status).

### W10 — post-flip features (separate tickets, unlocked by T-151.9)

**T-069** markers (+ line/area drawing on the polyline/polygon pipelines) · **T-070** vehicles
· **Ruler** (camera math, Class R vs hand-computed distances) · **LoS/viewshed** (DEM raymarch
in Rust over `sample_elevation_meters`, Class R vs brute-force reference sampler; viewshed as
WebGPU compute with the CPU raymarch as its exact oracle on sampled azimuths) · **T-071–T-075**
UI lane. Named deferred: binary chunk wire (human Workbench export), T-110 terrain deltas,
T-111 lazy doc residency, T-143 water, per-chunk anchors beyond Everon-size worlds.

## Sequencing rationale

Packaging (W0) unlocks zero-copy everywhere → basemap (W1) makes every later slice visually
verifiable in-editor → parser (W2) before residency (W3) → vectors (W4) are cheap wins (data
already Rust) → atlas (W5) gates slots (W6) which gate interaction (W7) → culling (W8) needs
real data volumes → flip (W9) last → features (W10) only on the flipped engine.

## Risk register (tripwire → response)

| Risk | Tripwire | Response |
|---|---|---|
| Bundler-target wgpu regression | W0 gate re-runs full spike self-check | Fallback: whole module on web target, one init site |
| `copyExternalImageToTexture` on WebGL2 | W1 corner probe | JS RGBA decode + `write_texture` |
| Triangulator correctness | W4 area-conservation gate | Earcut-port vs fan decided by the gate |
| Chunk parse hitching | W3 `stats()` budget measure | Worker resurrection (D2 reversible) |
| Glyph uniform table on WebGL2 | init assert (28 ≪ limits) | Texture-encoded UV table |
| Deck deletion breaking non-editor pages | W9 route-table E2E | — |

## Documentation sync (Cursor, after each slice merge)

Per-slice verify log `.ai/artifacts/t151_{n}_verify_log.md` (header records worktree toplevel +
HEAD SHA) → registry slice `shipped` + `shipped_at` → `./scripts/ticket sync` + `check` →
CLAUDE.md §Status bullet at program milestones. Registry `active_slice` stays **unset** on
T-151 (T-090 owns the `ACTIVE NOW` marker). Slice prompts are delivered manually by Cursor
(extractable via `./scripts/ticket prompt T-151 --slice T-151.n` for text only — never
`./scripts/ticket run`).
