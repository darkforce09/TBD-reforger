# Engine Split Program — graphics-engine / map-engine / editor app

**Status:** proposed, not yet ticketed.
**Scope:** `apps/website/` only. No mod (`apps/mod/`), no schema (`packages/tbd-schema/`), no API contract change.
**Shape:** three phases. Phase 1 is the expensive one; phases 2 and 3 are mostly mechanical.

---

## 0. Why

Today `apps/website/` holds four Rust crates and one of them is misnamed and doing three jobs:

| Crate | LOC | Actually contains |
|-------|-----|-------------------|
| `website-graphics-engine` | ~59k | ~50k map domain (terrain, spatial, streaming, symbology, environment, camera, formats) + ~9k actual renderer |
| `website-mission-core` | ~40k | `mission/` 18k (AST/compiler/validation, serde only, shared with api) + `doc/` 22k (yrs CRDT, 40-file row store, operations) |
| `website-frontend` | — | includes `src/editor/` 96k, of which ~18k is engine logic wearing a leptos coat |
| `website-api` | — | depends on `website-mission-core` (default `compiler` feature) |

Three consequences:

1. **The renderer is not a renderer.** `renderers/` and `core/` import `crate::terrain::*` and `crate::symbology::*` directly (~30 call sites, enumerated in §Phase 1). There is no frame/packet boundary — `grep 'struct \(FramePacket\|DrawBatch\|RenderState\)'` returns zero hits.
2. **Data management is split across three homes** — `mission-core/mission` (AST), `mission-core/doc` (CRDT rows), and `frontend/editor/state` (operations, history, persist) — all describing the same entities. Every entity change touches three places.
3. **`frontend/src/v2/map_engine/`** is four directories of READMEs and **0 lines of code**, pointing at a capability that already exists one crate over.

Target: **one data/world/editing engine, one pure renderer, one thin browser app.**

---

## 1. Target layout

This is the exact tree. Do not improvise on it.

```
apps/website/
├── api/                                    dep: map-engine {scenario}
│
├── graphics-engine/                        PURE wgpu. zero map nouns.
│   └── src/
│       ├── frame/                          DEFINES the vocabulary
│       │                                   FramePacket, DrawBatch, InstanceBuffer,
│       │                                   CameraUniform, TextRun
│       ├── device/                         ← core/{context,buffers}
│       ├── pipeline/                       ← core/pipeline + renderers/pipelines
│       ├── draw/                           ← renderers/{batching,engine,primitives}
│       ├── text/                           ← renderers/text + symbology/atlas
│       │                                     + symbology/instances/packing.rs   (SDF, glyph pack)
│       ├── loop/                           ← frontend editor/canvas/render_sync.rs (damage rAF)
│       ├── shaders/                        *.wgsl
│       └── diagnostics/                    ← diagnostics/{readback,timing,platform}
│
├── map-engine/                             data + world + editing. speaks frame vocab.
│   └── src/
│       ├── data/
│       │   ├── scenario/                   ast/ compiler/ validation/ extensions/        18k
│       │   └── store/                      crdt/ rows/ operations/ selection.rs          22k
│       ├── editing/                        ← frontend src/editor/state + tools          ~18k
│       │   ├── history/                    ← state/{history,doc_host}
│       │   ├── commands/                   command defs + dispatch (no keybinds)
│       │   ├── operations/                 ← state/operations  (merged into data/store/operations)
│       │   ├── tools/                      ← editor/tools — ruler, LOS, viewshed, select, place
│       │   └── persist/                    ← state/{persist,hydrate}  serialize only
│       ├── world/
│       │   ├── terrain/                    dem, relief, roads, satellite, water
│       │   ├── environment/                buildings, vegetation, locations, classify
│       │   └── architecture/               blueprint, compound, section
│       ├── spatial/
│       │   ├── bvh/  indexing/
│       │   ├── los/                        terrain/ · world/ · interior/
│       │   └── picking.rs
│       ├── camera/                         math, orbit, ortho
│       ├── streaming/                      scheduler, loaders, memory, buffers, host, bridge
│       ├── io/                             archives, containers, density, pod   (rkyv)
│       ├── overlay/symbology/              labels, roles, links, instances
│       ├── frame/                          BUILDS graphics_engine::frame packets
│       ├── doll/                           arsenal 3D — domain, consumes graphics-engine
│       └── diagnostics/                    bench, probes
│
└── frontend/src/
    ├── main.rs  router.rs  app_routes.rs
    └── v2/
        ├── core/                           api, auth, ui, utils, test_support
        ├── pages/                          command_center, operations, mission_hub,
        │                                   field_tools, doctrine_and_info,
        │                                   administration, navigation
        ├── apps/
        │   ├── editor/                                                                 ~55k
        │   │   ├── ui/                     docks, outliner, inspector, modals, toolbelt    43k
        │   │   ├── input/                  ← canvas/gestures + state/commands_hotkeys
        │   │   │                             pointer FSM, keybind map, drag-in-progress
        │   │   ├── bridge/                 ← canvas/{boot,viewport,overlays,gizmo_z}
        │   │   │                             leptos signal ↔ engine, canvas mount
        │   │   ├── shell/                  ← state/{tab_lock,save_status,title_prefer,session}
        │   │   └── arsenal/                loadout, catalog, rules, panels                12k
        │   ├── planner/
        │   ├── aar/
        │   └── debug/                      building_viewer, world_los testbenches
        └── map_engine/                     DELETED — 0 LOC, READMEs only
```

### Dependency direction — non-negotiable

```
api      ──► website-map-engine {scenario}                        native, no wgpu
frontend ──► website-map-engine {editing, store, world, render}   wasm
             website-map-engine ──► website-graphics-engine
```

`graphics-engine` never learns a map noun. `map-engine` speaks graphics' vocabulary, not the reverse.
`frontend` never imports `website-graphics-engine` — it hands `map-engine` a canvas handle.

### The rule that decides where state lives

> **Map engine owns state that survives a reload. Frontend owns state that dies with the tab.**

| survives → `map-engine` | dies → `frontend` |
|---|---|
| document, entities, undo stack | hover, drag-in-progress |
| selection | gesture/pointer FSM |
| tool command definitions | keybind map |
| LOS / ruler / viewshed algorithms | panel open/closed, dock sizes |
| serialize / hydrate | tab lock, save-status signal, title |

Apply this rule to any file the phases below do not name explicitly.

### Cargo features

> **Corrected 2026-09-16.** An earlier revision of this block had `world = ["io", …]`. That
> inverts the live chain, which is `formats = ["terrain", …]` — the on-disk format code depends
> on the terrain types, not the reverse. It was written without reading `Cargo.toml`. **A feature
> rename must not become a code change.** The block below preserves the existing direction.

```toml
# apps/website/map-engine/Cargo.toml
[features]
default   = ["scenario"]
scenario  = ["dep:serde", "dep:serde_json", "dep:thiserror"]                  # api uses this alone
store     = ["scenario", "dep:yrs"]
bvh       = []
world     = ["bvh", "dep:png"]                                                # was `terrain`
io        = ["world", "scenario", "serde_json/float_roundtrip", "dep:rkyv"]   # was `formats`
streaming = ["io", "dep:flate2"]                                              # own axis — keep it
render    = ["streaming", "dep:website-graphics-engine"]
editing   = ["store", "world"]
```

Three things this block is load-bearing about:

- **`streaming` stays its own axis.** Do not fold it into `world`. `frontend/Cargo.toml:27` takes
  `terrain` + `formats` with no streaming, and `xtask/Cargo.toml:34` takes all four. Real consumers
  already want the world/io pair without the scheduler; folding them forces 9.6k of scheduler,
  loaders and memory into builds that never call them.
- **The `default` flip is safe but must be proven.** Today's default is
  `["render","terrain","formats","streaming"]`. Four of the five manifest lines already pass
  `default-features = false`; only `frontend/Cargo.toml:110` rides the default, and its `["render"]`
  reaches the whole chain transitively. Build that target explicitly after the flip.
- **`scenario` / `store` edges are derived, not guessed.** Add them as the §2A fold demands and let
  the compiler say which of `render` / `editing` actually needs `store`. Pre-wiring an edge that
  turns out to be unnecessary will never warn you.

**Five manifest lines name these features and all five need editing** —
`frontend/Cargo.toml:{27,110,125}`, `xtask/Cargo.toml:34`, `tools/tbd-tools/Cargo.toml:31`. The
"leave `frontend:27/110/125` alone" note in §Phase 2's state table refers to the **dependency
path**, which Phase 1 already repointed. Feature-list edits on those lines are required.

Sizing: 157 `cfg(feature = …)` sites across the five renamed/retained features —
`render` 59, `streaming` 50, `formats` 38, `terrain` 5, `bvh` 5.

`wgpu` stays under `[target.'cfg(target_arch = "wasm32")'.dependencies]` in `graphics-engine`. The
server never compiles it. That is what makes `api → map-engine` a feature flag rather than a
coupling problem.

---

## 2. Phase 1 — graphics engine

**Goal:** a `website-graphics-engine` crate that compiles with zero knowledge of terrain, symbology,
Arma, or missions, consuming only its own `frame/` vocabulary.

This is the expensive phase. Do it first, while the source crate is 59k and not 99k.

### 1A — rename, then create

```bash
git mv apps/website/graphics-engine apps/website/map-engine
```

- Crate `website-graphics-engine` → `website-map-engine` in `apps/website/map-engine/Cargo.toml`.
- Update workspace members in root `Cargo.toml`.
- Repoint the **three** `website-graphics-engine` dep lines in `apps/website/frontend/Cargo.toml`
  (lines ~27, ~110, ~125 — each with a different feature set; preserve them exactly).
- Create new crate `apps/website/graphics-engine`, name **`website-graphics-engine`** (name reused
  deliberately — no long-term churn).

Acceptance 1A: `cargo build --workspace` green, no behavior change, nothing moved yet.

### 1B — define the vocabulary first

In `apps/website/graphics-engine/src/frame/`, before moving any renderer code:

```rust
pub struct FramePacket   // camera uniforms + ordered Vec<DrawBatch> + Vec<TextRun>
pub struct DrawBatch     // pipeline id, vertex/index buffer handles, instance range, bind group ids
pub struct InstanceBuffer
pub struct CameraUniform
pub struct TextRun       // glyph ids + positions, already packed — no label logic
```

Rule: these types may reference **geometry and GPU handles only**. If a field name contains
`terrain`, `road`, `symbol`, `label`, `town`, `contour`, or `sea`, it is wrong.

### 1C — move the mechanical half

Low-coupling first:

| From (`map-engine/src/`) | To (`graphics-engine/src/`) |
|---|---|
| `core/context/`, `core/buffers/` | `device/` |
| `core/pipeline/`, `renderers/pipelines/` | `pipeline/` |
| `shaders/*.wgsl` | `shaders/` |
| `diagnostics/{readback,timing,platform}/` | `diagnostics/` |

`core/culling/` — **decide and record**: frustum culling needs world bounds, so it most likely
belongs to `map-engine/spatial/`. Do not move it to graphics by default.

### 1D — move the coupled half and invert the imports

`renderers/{batching,engine,primitives}` → `draw/`; `renderers/text` + `symbology/atlas` +
`symbology/instances/packing.rs` → `text/`.

These are the actual call sites that must be inverted. Each one becomes: **map-engine computes it
and puts the result in a `DrawBatch`/`TextRun`.**

From `renderers/`:
```
crate::terrain::roads::mesh::compose_roads_mesh            (4 sites)
crate::symbology::instances::symbols::*                    (3)
crate::terrain::water::mesh::compose_sea_mesh              (2)
crate::terrain::relief::contours::*                        (2)
crate::symbology::labels::declutter::declutter             (2)
crate::terrain::satellite::textures::*
crate::terrain::relief::sea_band::build_sea_band_geometry
crate::terrain::dem::grid::*
crate::symbology::labels::importance::{town_label_fade_alpha, declutter_town_labels}
crate::symbology::labels::glyph_math::{size_with_min_px, pack_rgba_u, pack_icon_instance}
crate::symbology::instances::packing::{pack_rgba_u, pack_icon_instance}
```

From `core/`:
```
crate::symbology::instances::bridge_*                      (3)
crate::symbology::instances::symbols::cluster_mode
crate::symbology::instances::lanes::*
crate::symbology::atlas::gpu::*
crate::terrain::satellite::textures::*
```

**Symbology splits across the boundary and both halves are real:**

- `symbology/atlas/` (`gpu.rs`, `raster.rs`) + `symbology/instances/packing.rs`
  (`pack_rgba_u`, `pack_icon_instance`) are mechanical bit-packing → **graphics-engine/text/**
- `symbology/{labels,roles,links}` and the rest of `instances/` decide *which* symbol and *whether
  it is decluttered at this zoom* → stays **map-engine/overlay/symbology/**

### 1D.1 — `LaneRole` goes opaque

> **Added 2026-09-16.** The original spec missed this. It is the largest single item in Phase 1.

`core/pipeline/draw_order.rs` defines `LaneRole` with 48 variants; `core/pipeline/roles.rs` defines
24 `role_id` consts plus 2 `tex_role_id` consts, named after map features (`SEA`, `CONTOURS`,
`ROADS_CASING`, `LANDCOVER`, `FOREST_OUTLINE`, `AIRFIELD_APRON`, `INTERIOR_PROBE`, `MISSION_ZONES`,
`HILLSHADE`, `BASEMAP`). `LaneRole` is referenced by **24 files** and is the renderer's spine —
buffer pools, batching, culling, text lanes, selection, bench all key on it.

Five of those names contain `mission` and trip gate rule 2 outright: `role_id::MISSION_ZONES`,
`LaneRole::{MissionVehicles, MissionMarkers, MissionComments, MissionConnections}`. Keeping the
names verbatim is not an option.

**Design:**

```
graphics-engine:            pub struct LaneId(u32);     // Copy, Hash, Ord, opaque
                            pools / batching / culling / text lanes key on LaneId.
                            Paint order arrives as an explicit u8 sort key on the
                            batch, or the batch list arrives pre-ordered.
                            Graphics never ranks lanes.

map-engine/overlay/lanes.rs the 48 named identities and their order — one source of
                            truth. `pub const SEA: LaneId = LaneId(0);` …
```

`lane_order()` **has no production caller** — its only caller is
`core/pipeline/tests/draw_order.rs`. Declaration order is the real paint order; `lane_order()` is
the cartographic spec written as an assertion. It and its test move to `map-engine` with the names.
This makes the "48-variant ordering table" far cheaper than it looks: it is not renderer machinery.

**Sizing:** `rg 'role_id::'` → 82 sites in `graphics-engine/src`, 76 in `frontend/src`. Most of the
82 sit in files that leave for `map-engine` in Phase 2 anyway (`terrain/satellite/textures.rs`,
`symbology/instances/*`, `environment/{vegetation,buildings}/buffers.rs`,
`spatial/terrain_los/overlay.rs`). Only sites remaining in `renderers/`, `core/{buffers,culling,
context}/`, and `diagnostics/bench/` must actually go opaque. Measure that stay-behind count against
the post-1C/1D split before executing; if it exceeds ~40, stop and raise it with the operator.

**Zero behavior change:** same `u32` values, same sequence, same draw order. The `draw_order` tests
must pass unchanged after the move.

### 1E — collapse the rAF pump

> **Corrected 2026-09-16.** An earlier revision of this document named
> `canvas/render_sync.rs` as the frame loop. That is wrong — `render_sync.rs` contains zero
> rAF code (`grep -c raf` → 0). It is the T-934.10 pure helper belt (pick/lane/hover
> helpers over `serde_json`). Leave it alone in Phase 1; see §Phase 3.

The frame pump is `apps/website/frontend/src/editor/canvas/viewport.rs::start_raf` (384-line file).
**There are three copies of it**, and collapsing them is the point of this step:

```
editor/canvas/viewport.rs::start_raf
pages/debug/building_viewer.rs:2150      "the editor's `start_raf` idiom"
pages/debug/world_los.rs:135             "the editor's `start_raf` minus the HUD"
```

`graphics-engine/src/loop/` ships **one** reusable `RafPump` — rAF tick, render call, engine poll,
frame counting, disposal check — and all three call sites use it. `web_sys::request_animation_frame`
is fine here: `graphics-engine` is already a wasm crate with `web-sys`. Gate rule 5 forbids DOM in
`map-engine`, not in `graphics-engine`.

What stays in the frontend: the leptos half — `debug_hud`/`scale_mm` writes, the write-on-change
guard, `device_size`, the `window.*` registrars, `registry_session`.

**Keep `fn start_raf(` in `viewport.rs`.** `mission_editor_tests/t670_scale_signal.rs:52` scrubs
that file for that exact string. The leptos half lives there anyway — let it keep the name and call
the pump. Do not move the pin.

Real damage tracking already lives engine-side at `core/pipeline/damage.rs` and moves with `1C`.

### 1F — build the wall

Add gate rule (see §5):

```
apps/website/graphics-engine/**  may not import  website_map_engine
```

### Phase 1 acceptance

- `cargo build -p website-graphics-engine --target wasm32-unknown-unknown` green.
- `rg 'terrain|symbology|mission|orbat|arma' apps/website/graphics-engine/src --type rust -i`
  returns only shader filenames and comments — **no type or function names**.
- `cargo xtask ci ci-local` green.
- `cargo xtask mk ci-local-leptos` green.
- `cargo xtask mk leptos-gates` green (run `gate doctor` first — see
  [`EDITOR_GATE_RUNBOOK.md`](../website/EDITOR_GATE_RUNBOOK.md); needs full Chrome `--headless=new`).
- Editor renders identically. HUD `rf <ms>` within noise of pre-split; capture
  `window.__editorBench(500)` before and after and put both in the verify log.

---

## 3. Phase 2 — map engine

**Goal:** one crate owning all mission data, all world data, all spatial query. `website-mission-core`
ceases to exist.

### Post-Phase-1 state — do not re-derive

Measured after 1F landed. Phase 2 starts from here, not from the tree §0 describes.

```
apps/website/graphics-engine    4.5k   device/ draw/ frame/ loop/ pipeline/ shaders/ text/
apps/website/map-engine        56.6k   architecture 6113 · camera 852 · core 3163
                                       diagnostics 3078 · doll 1517 · environment 5707
                                       formats 3518 · renderers 1974 · spatial 7413
                                       streaming 9624 · symbology 5514 · terrain 7932 · world 171
apps/website/mission-core      ~40k    still standing, still depended on
```

| Landmark | Now at |
|---|---|
| `LaneId(pub u16)` | `graphics-engine/src/frame/ids.rs` |
| damage tracking | `graphics-engine/src/frame/damage.rs` |
| named lane identities | `map-engine/src/core/pipeline/{draw_order,roles}.rs` |
| api → mission-core | `apps/website/api_v2/Cargo.toml:62` |
| frontend → mission-core | `apps/website/frontend/Cargo.toml:11` |
| frontend → map-engine | `apps/website/frontend/Cargo.toml:27, 110, 125` — **path already repointed; do not touch the path. The feature lists on these lines DO change** (see §1 Cargo features) |
| other map-engine consumers | `xtask/Cargo.toml:34`, `tools/tbd-tools/Cargo.toml:31` — five manifest lines total name map-engine features |

### 2A — fold mission-core in

| From | To |
|---|---|
| `mission-core/src/mission/{ast,compiler,validation,extensions}` | `map-engine/src/data/scenario/` |
| `mission-core/src/doc/crdt` | `map-engine/src/data/store/crdt/` |
| `mission-core/src/doc/store` (40 files) | `map-engine/src/data/store/rows/` |
| `mission-core/src/doc/operations` | `map-engine/src/data/store/operations/` |
| `mission-core/src/doc/picking` | `map-engine/src/data/store/selection.rs` |
| `mission-core/src/slot_line` | `map-engine/src/data/scenario/slot_line/` |

Merge the feature sets per §1 Cargo. Repoint `apps/website/api_v2/Cargo.toml:62`
(`website-mission-core` → `website-map-engine`, default features). Repoint
`apps/website/frontend/Cargo.toml:11`. Delete `apps/website/mission-core/` and its workspace member.

The term **"mission doc" is retired.** It was never a concept — it is the `yrs` layer. It becomes
`data/store/` and gets no box, no crate, and no name of its own.

### 2B — reshape internals

| From | To |
|---|---|
| `terrain/` | `world/terrain/` |
| `environment/` | `world/environment/` |
| `architecture/{blueprint,compound,section}` | `world/architecture/` |
| `architecture/los/` | `spatial/los/interior/` |
| `spatial/terrain_los/` | `spatial/los/terrain/` |
| `spatial/world_los/` | `spatial/los/world/` |
| `formats/` | `io/` |
| `symbology/` (minus atlas + packing.rs) | `overlay/symbology/` |
| `core/pipeline/{draw_order.rs,roles.rs}` | `overlay/lanes.rs` — per §1D.1; the named identities and their order belong with the cartography, not under `core/pipeline/` |
| `diagnostics/{bench,probes}` | `diagnostics/` |

The three LOS trees are **layered, not duplicated** — heightfield march, BVH/TLAS over objects,
building interiors. Do not attempt to merge them. Just make them adjacent.

Two traps in this step:

- **`world/` already exists** (171 LOC — `mod.rs`, `scene.rs`, `tests`, seeded during Phase 1).
  Merge into it. Do not clobber it.
- **`map-engine/src/target-container/`** is a gitignored local build directory that sits inside
  `src/`. Exclude it from every bulk move and every `find` / `rg` sweep. Never commit it.

New: `map-engine/src/spatial/picking.rs` — single hit-test entry point. Returns **ids**.
`data/store/selection.rs` turns ids into selection. Neither does the other's job.

### 2B.1 — place the 5.1k the target tree does not name

> **Added 2026-09-16, after Phase 1 measurement.** §1's target tree was drawn before Phase 1 ran
> and assumed `core/` and `renderers/` emptied completely into `graphics-engine`. They did not,
> and correctly so.

`map-engine/src/core/` (3163) and `map-engine/src/renderers/` (1974) survived Phase 1 as
map-engine's own half of the split: batch building, culling, context preferences, and the
lane-building belts (`renderers/batching/lanes.rs`, `renderers/text/lanes.rs`). The target tree in
§1 has no `core/` or `renderers/` under `map-engine`. Place them:

| Content | Destination |
|---|---|
| packet / batch construction | `map-engine/src/frame/` |
| culling | `map-engine/src/spatial/` |
| anything else | apply the reload rule; **state where it went and why** |

**Report the mapping before moving a single file.** This is the one part of Phase 2 that is not
specified in advance. Unplaced code during a phase of bulk moves is how things land in the wrong
crate without anyone noticing.

**Do §2C.1 first.** Twenty of these files are Kind A there — GPU resource creation that belongs in
`graphics-engine`, not anywhere in `map-engine`. Sorting that out shrinks this mapping before you
have to make judgement calls about the rest.

### 2C — build `map-engine/src/frame/`

Does not exist yet. It builds `graphics_engine::frame::FramePacket` from world + data + overlay,
and it is the **only** module in `map-engine` allowed to name a graphics type.

**Four rules. Violating any one costs frames, and nothing in the test suite will catch it** — only
the bench will show it, and only if someone reads the bench.

1. **Persistent packet.** `clear()` and refill, keeping capacity. Never allocate a fresh
   `Vec<DrawBatch>` per frame — that is per-frame `dlmalloc` churn at 60 Hz. **The case that
   matters is the one that scales with scene complexity** — the packet's batch vec — not the
   fixed-size lookup tables; see §2C.2.
2. **Handles and ranges only.** A `DrawBatch` carries a buffer handle and an instance range. The
   moment it carries owned `Vec<f32>` geometry, you have added a per-frame copy of the geometry.
3. **Damage tracking must survive.** `graphics-engine/src/frame/damage.rs` exists. If packet
   building becomes "walk the world, rebuild every batch," you have turned a damage-driven renderer
   into an immediate-mode one. That is the whole design gone, not a few percent.
4. **One call per frame across the crate boundary.** `render(&FramePacket)`. Not per-batch, not
   per-instance — without LTO, a chatty boundary is thousands of un-inlined calls per frame.

Rule 3 is the one to verify before the other three. It is also the only one with an existing
instrument: the Phase 1 `__editorBench(500)` baseline.

### 2C.1 — "only `frame/` may name a graphics type" — what that actually means

> **Added 2026-09-16.** The sentence above is too loose on its own. Measured: **39 files** in
> `map-engine/src` name `website_graphics_engine` today, across **six** graphics modules —
> `frame`, `draw`, `text`, `shaders`, `device`, `pipeline`, `r#loop`. They are three different
> kinds and a single rule cannot cover them. Twenty of the 39 are the unplaced 5.1k from §2B.1;
> this step and that one are the same question.

**Kind A — GPU resource creation that Phase 1 under-moved. Relocate, do not chokepoint.**

```
core/context/device_2.rs:113   builds a wgpu shader module from shaders::SHADER_WGSL
symbology/atlas/gpu.rs:37      calls text::gpu::create_glyph_atlas
renderers/text/lanes.rs:43     calls text::gpu::create_text_atlas
renderers/engine/lifecycle.rs  text::gpu::TEXT_UNIFORM_BYTES
core/mod.rs:9                  pub use device::buffers
renderers/mod.rs:16            pub use pipeline as pipelines
renderers/engine/pump.rs       r#loop::{FrameTarget, RafPump}
```

Device, shader-module, atlas and pipeline creation are `graphics-engine` responsibilities. These
move there. Most of §2B.1's unplaced `core/` + `renderers/` is this. Do Kind A **first** — it is
the only one of the three that carries behavior risk.

**Kind B — POD layout and bit-packing. Legitimate direct imports; leave them where they are.**

```
draw::instances::{QuadInstance, IconInstance, BuildingInstance, UNIT_QUAD,
                  CHUNK_CAPACITY, ATLAS_GLYPH_COUNT}
draw::geometry::{LineVertex, pack_offset, corner_uv}
draw::encode::encode · draw::compose::retint_fill_alpha
text::metrics::* · text::layout::*
```

This is the shared binary contract between the two crates, not graphics behavior. Its consumers
are upload belts that sit next to their data — `environment/{buildings,vegetation}/buffers.rs`,
`symbology/instances/*`, `world/scene.rs`, `terrain/satellite/textures.rs`. Routing them through
`frame/` would put vegetation knowledge inside `frame/` and make it a god-module.

Expose them from graphics-engine as **one named surface, `graphics_engine::layout`**, so the ABI
reads as a list. Belts import it directly. Gate rule 3 does not restrict it.

**Kind C — the packet vocabulary. This is what the rule was about.**

```
frame::{FramePacket, DrawBatch, DrawPayload, InstanceBuffer, LaneId, CameraUniform}
```

Chokepoint through `crate::frame`, re-exported **explicitly and enumerated — never a glob**. The
value is that map-engine's entire graphics interface surface is a readable list in one file.

Report the Kind A/B/C split with file counts before moving anything.

### 2C.2 — the rule-1 violation Phase 1 shipped

> **Added 2026-09-16.** Found during Phase 2 planning. Phase 1 introduced it; it is inside the
> `__editorBench` baseline, so the baseline normalizes it.

`renderers/batching/encoder.rs::encode_main_pass` rebuilds two lookup tables on every frame:

```
pipeline_table()     Vec::with_capacity(PIPELINE_SLOTS = 9)   + 9 Arc clones
bind_group_table()   vec![None; BIND_SLOTS = 100]             + ~5..N Arc clones
```

Two heap allocations and roughly twenty atomic refcount operations per frame. Introduced in
`24e22b1b9` — *"invert the lanes — the encoder stops asking what a lane is (1D, part 3)"*. The
doc comment on `pipeline_table` already states *"Cloning is an `Arc` bump per pipeline per frame"*:
the pattern was seen and shipped. That comment is precisely the signal that should have triggered
a hoist at the time.

**Fix it when `encoder.rs` becomes `frame/encode.rs`.** Hoist both to `RenderEngine` fields,
`clear()` + refill.

Two things to be honest about while doing it:

- **It will not move the bench.** Under a microsecond against a 16.6 ms budget — below 0.01% of
  frame time. Landing inside noise is the *expected* result here, not evidence the hoist failed.
  The reason to fix it is that `frame/encode.rs` is the module that sets the packet-building
  pattern for everything downstream; the canonical rule-1 violation living inside the canonical
  rule-1 module is how the rule stops being real.
- **These tables are not the rule-1 case that matters.** Both are fixed-size — 9 and 100,
  independent of scene complexity. Rule 1 is about the packet's `Vec<DrawBatch>`, which scales
  with what is on screen. Check that one first and report whether it is hoisted or rebuilt.

Expect a borrow fight: hoisting `bind_group_table` means holding `&mut self` while reading
`self.{glyph_atlas, text_atlas, slot_atlas, tex_lanes}`. **Do not reach for `RefCell`.**
`mem::take` the field, refill, put it back — or keep the builder a free function taking an
`&mut Vec` out-param, which is what `pipeline_table` already is.

This is the **only** runtime-behavior edit in Phase 2. Give it its own commit, with a message that
says why it is not a pure move.

There is an upside to claim here, not just a regression to avoid. Before Phase 1, `renderers/`
imported `compose_roads_mesh`, `compose_sea_mesh`, `build_sea_band_geometry` and `contours` —
mesh composition sat *in the draw path*. Building it on the map-engine side during packet
assembly lets it be cached and damage-gated independently of drawing. If any of that was running
per frame for data that never changed, this phase is a speed-up.

### 2D — separate static world from authored entities

`world/` is immutable, streamed from `packages/map-assets`, cacheable, never persisted.
`data/` is mutable, undoable, CRDT-synced, persisted. They share the spatial index and nothing else.
Do not let a `world/` type gain a `dirty` flag or a `data/` type gain a chunk id.

### Phase 2 acceptance

- `cargo build -p website-api` green and **does not compile** `world/`, `streaming/`, `render/`, or
  `wgpu`. Verify: `cargo tree -p website-api | rg -i 'wgpu|png|rkyv|flate2'` → empty.
- `cargo xtask db up && cargo xtask db test-it` green.
- `cargo test -p website-frontend` green — R-api golden round-trips unchanged. **The API JSON
  contract does not move in this program.** snake_case stays snake_case; the
  `/missions/:id/export` camelCase exception stays camelCase.
- `io/` byte format unchanged: rkyv stays `little_endian` + `bytecheck`. Existing `.bvh` sidecars
  and committed map-asset fixtures still load. Run `cargo xtask verify-terrain`,
  `verify-map-object-golden`, `verify-oracle`.
- Generated contract files under `apps/website/api_v2/src/contract/generated/` untouched
  (DO NOT EDIT) — `cargo xtask ci schema-codegen` then `verify-codegen-fresh` still green.
- `cargo xtask verify-engine-layers` green — rules 1 and 2 still hold after the reshape, and
  rules 3a/3b are now implemented. **Rule 3b must report zero sites**; it fails today
  (39 files, six graphics modules — see §2C.1), and closing it is Phase 2 work.
- `cargo xtask ci ci-local`, `cargo xtask mk ci-local-leptos`, `cargo xtask mk leptos-gates`
  (run `gate doctor` first) all green.
- **`__editorBench(500)` against the Phase 1 baseline, frame time within noise.** This is the
  only instrument that can detect a §2C rule-3 violation. A green test suite proves nothing here.

Paste every acceptance output verbatim. Do not summarize them.

---

## 4. Phase 3 — app and editor

**Goal:** `frontend/src/editor/` 96k becomes `v2/apps/editor/` ~55k; the other ~18k moves into
`map-engine/editing/` and ~22k of duplication collapses.

### 3A — pull editing into the engine

| From (`frontend/src/editor/`) | To (`map-engine/src/editing/`) |
|---|---|
| `state/history.rs`, `state/doc_host.rs` | `history/` (drives `data/store/crdt/undo_groups`) |
| `state/operations/` | **merge into** `data/store/operations/` — see below |
| `state/{persist.rs,hydrate.rs}` | `persist/` (serialize only) |
| `tools/{los_tool,los_world,ruler_tool,select_tool,viewshed_scheduler,place_helpers}` | `tools/` |
| `state/commands_hotkeys.rs` — **command half only** | `commands/` |

Two dedupes fall out and both are required, not optional:

1. **`editor/state/operations/` vs `doc/operations/`** share eight filenames (`attrs`, `cargo`,
   `compositions`, `reassign`, `slot_ids`, `tactical_graphics`, `transform`, `entity/`). Audit first:
   if the frontend copies are thin adapters, delete them; if they are a second implementation,
   reconcile into `data/store/operations/` and keep the behavior the editor currently ships.
2. **`editor/tools/los_world.rs` + `los_world_wasm.rs`** duplicate `spatial/los/world/`. Collapse
   into the engine copy.

Also delete: `editor/state/picking/` is a README and a `tests/` directory with no implementation —
it already migrated. Move the tests to `data/store/selection.rs`, delete the husk.

Tool FSMs move **without their DOM**. An `editing/tools/` type may hold `armed | active | committed`
and the geometry; it may not hold a `PointerEvent`, a leptos signal, or an element ref.

### 3B — reshape the frontend

| From (`frontend/src/editor/`) | To (`frontend/src/v2/apps/editor/`) |
|---|---|
| `panels/{dock_left,dock_right,top_strip,toolbelt,context_menu}` | `ui/docks/` |
| `panels/{outliner,outliner_tree,outliner_drag}` | `ui/outliner/` |
| `panels/{attributes_modal,env,weather_timeline,zones_panel,vehicles_panel,radio_panel,tasks_panel,spawn_modules,audio_emitters,win_conditions_card,validation_panel}` | `ui/inspector/` |
| `panels/{help_modal,settings_modal}` | `ui/modals/` |
| `canvas/gestures.rs` + `state/commands_hotkeys.rs` (keybind half) | `input/` |
| `canvas/{boot,viewport,overlays,gizmo_z,commands,tactical_graphics}` | `bridge/` |

**`canvas/render_sync.rs` (998 lines) splits by the reload rule — this is the one Phase 1
deliberately left alone.** It is misnamed: it holds no render sync, it is a pick/lane helper belt.

| From `render_sync.rs` | To |
|---|---|
| `connection_segments`, `connection_lane_verts`, `pick_connection` | `map-engine/editing/` |
| `comment_points`, `comment_lane_xy`, `comment_lane_ids`, `pick_comment`, `dragged_comment_points`, `comment_drag_lane_xy` | `map-engine/editing/` |
| `marker_lane_fields`, `route_target`, `route_availability`, `zone_centre` | `map-engine/editing/` |
| `selectable_ids`, `crewed_slot_ids`, `map_render_keep_indices`, `map_render_slot_soa`, `filter_slot_soa_excluding`, `plain_paste_anchor` | `map-engine/editing/` |
| `hover_due`, `hover_next`, `hover_cursor_css`, `hover_suppressed` | `frontend .../editor/bridge/` (tab-local) |

This move **breaks three source-scrubbing tests** that `include_str!` the file by relative path —
`t802_hover_cursor.rs:21`, `t808_symbology_feed.rs:23`, `t784_comment_glyph.rs:33`. Update those
pins in Phase 3; they are expected breakage, not a regression.

Its one engine touch, `website_graphics_engine::symbology::roles::classify::side_rgba` (line ~320),
resolves to `map-engine/overlay/symbology/` after Phase 1 — no action needed.

`mission_editor.rs` re-exports the `render_sync` belt `pub(crate)` under the same names at lines
~55, ~65 and ~3088. Those re-exports are the seam — repoint them, do not delete them blind.
| `state/{tab_lock,save_status,title_prefer,session}` | `shell/` |
| `arsenal/` | `arsenal/` (`panels.rs` → `ui/arsenal/`) |
| `mission_editor_tests/` | `tests/` |
| `eden_chrome.rs`, `layout.rs`, `mission_size.rs`, `world_layer_prefs.rs` | apply the reload rule |

Delete `frontend/src/v2/map_engine/` — four directories, four READMEs, zero lines.

#### The two unmigrated trees still under `frontend/src/pages/`

Neither is residue of its v2 namesake. Both are live code that never migrated.

| From | To | Why |
|---|---|---|
| `pages/operations/orbat_manager.rs` (2128), `faction_manager.rs` (459) | `v2/apps/editor/ui/modals/` | **Not pages.** Never routed; mounted only from inside the editor — `mission_editor.rs:2848` mounts `FactionManagerDialog`, `eden_chrome.rs:39` re-exports `OrbatManagerDialog`. Zero content overlap with `v2/pages/operations/`; only `mod.rs` shares a name. A non-routed editor dialog living under `pages/` is exactly the naming failure CLAUDE.md Law 4 targets. |
| `pages/debug/{building_viewer,world_los,building_interior,world_los_scene}` (4730) | `v2/apps/debug/` | Genuinely routed — `router.rs:161` `/debug/building-viewer`, `router.rs:169` `/debug/world-los`. `v2/apps/debug/` is README-only scaffolding today. |

`orbat_manager.rs` (2128) and `building_viewer.rs` (2715) exceed the Law 7 ceiling and are split in
§3C, not during the move.

**`orbat_manager` and `v2/pages/operations/orbat_selection/` are not duplicates.** One authors an
ORBAT inside the editor; the other is the routed slotting view where people sign up. They stay
distinct — the shared noun is not a merge candidate.

**Pin sweep before moving either tree.** These files are held by source-scrubbing tests, including
one outside the editor: `v2/core/ui/tests/ui.rs:471` does
`include_str!("../../../../pages/operations/orbat_manager.rs")`, and `help_modal.rs:679,684` pin
both files for the keymap census. This codebase pins by source inspection routinely —
`editor/panels/toolbelt.rs:948` says so explicitly. Sweep the **whole frontend crate** for
`include_str!`, relative-path literals, and tests naming these files. Do not hand-enumerate from
`editor/` alone.

Reconcile `frontend/src/v2/README.md` with this document. Its `map_engine/` section is now false.
Its `apps/{editor,planner,aar}` section is correct and this layout preserves it — `planner/` and
`aar/` stay empty scaffolds, but `map-engine/editing/` must not acquire editor-only assumptions
that would block them.

### 3C — CLAUDE.md Law 7: the 500-line ceiling, enforced repo-wide

> **Added 2026-09-16.** Law 7 postdates this document's Phase 3 section. It is the larger half
> of the phase, and by operator decision the gate ratchets **repo-wide**, not just over the paths
> Phase 3 touches.

Law 7 has **two** thresholds, and conflating them inflates the work by a third:

```
production files   under  500 LOC
test files         under 1000 LOC
```

A file is a test file when its path contains `/tests/` or its name ends `_tests.rs` — this
repo's convention, since Law 7 also bans inline `#[cfg(test)] mod tests` blocks in favour of
sibling files declared `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.

#### The work Phase 3 itself does

**43 files under `editor/` exceed 500 LOC; 25 exceed 1000.** Worst: `dock_right` 6456,
`top_strip` 4759, `settings_modal` 4510, `attributes_modal` 4082, `mission_editor` 3735,
`dock_left` 3115, `asset_catalog` 3052, `loadout` 2973, `arsenal_rules` 2959. Add the four from
§3B — `building_viewer` 2715, `orbat_manager` 2128, `world_los` 683, `building_interior` 541.

Those 47 are the whole of the frontend's debt. **After 3C the frontend crate is at zero**, and
gets no grandfather rows at all.

**Do 3C after 3A and 3B land green.** Move first, split second. A 6456-line file that both moves
and splits in one commit is unreviewable and destroys bisect.

Splitting obeys **Law 5** — group variants into named subfolders; never flat-dump
`dock_right_1.rs … dock_right_13.rs`. And **Law 4** — every new filename self-describing with
zero project context.

#### The gate: ratchet, don't sweep

`cargo xtask verify-file-length` (in `xtask/src/node_free.rs`) today has **SIZE-1 warn >600** and
**SIZE-3 fail >1000**, with no 500 rule anywhere. Law 7 is unenforced.

Repo-wide state, production vs test counted separately:

| tree | prod >500 | test >1000 |
|---|---|---|
| `map-engine` | **0** | **0** |
| `graphics-engine` | **0** | **0** |
| `frontend` | 47 — all cleared by 3C | 0 |
| `api` | 19 | 6 |
| `tools/tbd-tools` | 21 | 0 |
| `ticketboard` | 8 | 0 |
| `crates` | 5 | 0 |
| `xtask` | **73** | 0 |

179 violations; **132 remain once 3C clears the frontend**, and `xtask` is 73 of them.

The allowlist already supports a ratchet. `node_free.rs::exemption_fields_ok` requires a
non-empty `reason` **and** `expires_ok`, and `expires` must be a real `YYYY-MM-DD` that has not
passed. **Expired rows stop exempting.** So:

1. **Make `SIZE-3`'s threshold file-kind aware** — 500 for production, 1000 for test. One rule,
   two thresholds, keyed on the path test above. The existing 45 rows keep working, including
   the ones covering `api`'s six oversized test files.
2. **Retire `SIZE-1`.** Its >600 warn is dead once 500 hard-fails.
3. **Generate ~132 grandfather rows** for the current offenders, each with a real `reason` and a
   **dated** `expires`. Stagger them — 132 rows expiring on one day is a single red wall:

   | trees | suggested `expires` |
   |---|---|
   | `api`, `tools/tbd-tools`, `crates`, `ticketboard` | `2027-01-31` |
   | `xtask` | `2027-06-30` — largest holder, tooling not shipped code |

   **Never use `expires: MC-perf` for a grandfather row.** That is the never-expires escape
   hatch; using it here makes the debt permanent by construction.
4. **Delete the 29 obsolete rows** for files 3C split — per file, never by path prefix. A row
   whose path is gone is dead; a row whose content moved somewhere still oversized is
   load-bearing.

From the moment the threshold flips: a new file over 500 fails with no row to hide behind, a
grandfathered file that grows still fails, renewing a row shows up in review, and the backlog
drains on a date rather than on goodwill. No path-scope list to maintain, no gap where
`data/store/operations/` escapes, and `planner/` and `aar/` inherit the rule by existing.

Unrelated, low priority: a file-length gate living in `node_free.rs` is itself a Law 4 failure.
Note it; do not fix it here.

### Phase 3 acceptance

- `cargo xtask mk ci-local-leptos` green.
- `cargo xtask verify-file-length` green with the new 500 tier active on the scoped paths.
- `cargo xtask mk leptos-gates` green **except `gate v-suite verify`** — see §3C.1.
- `cargo xtask mk leptos` serves; dev-login → editor opens, place/select/move/undo/redo/save all work.
- Headless probe sweep per [`render-check`](../mod/MCP_TOOLING.md) editor probes: `--seed-auth`,
  `--map-assets`, LoS button via `pointerdown`, `camSet` + centre click.
- `rg 'web_sys|leptos|wasm_bindgen' apps/website/map-engine/src/editing` → empty.
- Zero production files over 500 LOC under `v2/apps/editor/` (CLAUDE.md Law 7); no inline
  `#[cfg(test)] mod tests` blocks in anything Phase 3 touches.
- Editor frame time within noise of the Phase 2 baseline.

### 3C.1 — `gate v-suite verify` is red at baseline; do not fix it here

> **Added 2026-09-16.** Pre-existing and unrelated to the engine split. Filed as
> [`T-986`](../../.ai/tickets/T-986.toml), status `idea`.

The frozen route oracles at `tools/tbd-tools/fixtures/t159/oracle-freeze` were last refreshed by
`dddf31581` (2026-07-18, T-173). The pages have moved repeatedly since, so `gate v-suite verify`
fails **22 of 25 routes**; four are clean.

**T-986 is deferred out of this program by operator decision. Do not re-freeze any oracle during
Phase 3.** The reason is not scope — it is that these oracles are the only instrument checking
route DOM output, and Phase 3's entire contract is zero behavior change. Re-freezing them inside
the phase bakes any regression Phase 3 introduces into the new oracle, where it becomes
permanently invisible. Re-freezing *only the routes Phase 3 moves* is worse still: that is
precisely the subset where the stale oracle retains diagnostic value.

Acceptance is a baseline diff instead:

1. Capture `gate v-suite verify` on `HEAD` **before the first Phase 3 commit**, and **commit the
   capture as a file** — a pasted log is not evidence anyone can re-check later.
2. Compare **route by route, not by count.** Twenty-two failures before and twenty-two after can
   be a different twenty-two. The acceptance is set equality on failing route names: unchanged or
   smaller. Fail → fail is fine. **Pass → fail is a hard stop.**
3. **The four clean routes are the real gate.** Name them explicitly in the captured baseline.
   They must still pass — hard fail, not diff.

Unrelated free cleanup: `oracle-freeze` still holds `content.react.dom.json` and
`missions.react.dom.json`. The React app was deleted at T-159.29.3, so these describe something
that no longer exists and cannot be re-frozen. Delete them. That is not T-986 work.

---

## 5. Gate rules

One 99k crate rots without enforcement. These replace the crate walls the design deliberately
does not build. Implement as a new `xtask` verify subcommand backed by `crates/tbd-gate`
(`pattern.rs` + `scan.rs` + `verdict.rs` already do this shape — see the existing
`verify-coding-standards` and `verify-no-node` for the pattern):

```
cargo xtask verify-engine-layers
```

| # | Rule | Guards |
|---|---|---|
| 1 | `graphics-engine/**` may not import `website_map_engine` | the hard wall |
| 2 | `graphics-engine/**` may not contain `terrain`, `symbology`, `mission`, `orbat`, `arma` as a **case-insensitive substring** of any type, fn, const, enum-variant, or module name | keeps "pure" honest |
| 3a | Only `map-engine/src/frame/**` may name `website_graphics_engine::frame` | keeps the packet boundary at one readable seam |
| 3b | **No** module in `map-engine` may name `website_graphics_engine::{device, pipeline, shaders, text::gpu, r#loop}` | GPU resources belong to graphics-engine. After §2C.1 Kind A moves, this must read **zero** |
| 4 | `map-engine/data/scenario/**` imports nothing outside itself | keeps the `api` build thin |
| 5 | `map-engine/**` may not import `web_sys`, `leptos`, or `wasm_bindgen` | keeps engine headless-testable |
| 6 | `frontend/**` may not import `website_graphics_engine` | one arrow, not two |

Rule 5 is the one that stops `editing/` drifting back into the browser. Wire
`verify-engine-layers` into `cargo xtask ci ci-local` and `.github/workflows/ci.yml`.

---

## 6. Execution rules

- **Read [`CLAUDE.md`](../../CLAUDE.md) first.** The HARD GATE applies: do the whole ask. No
  "folded forward", no agent-authored Out-of-scope, no verify-log DEFERRED section in place of code.
  Only an explicit operator "defer X" defers anything.
- **Commit directly to `main`. Never create a branch.** Tag each phase `T-0xx`. End commit messages
  with the `Co-Authored-By` trailer.
- **One phase per commit series.** A phase does not land until its acceptance list is green.
  Do not start Phase 2 on a red Phase 1.
- Add a registry row per phase in [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json),
  then `cargo run -q -p xtask -- ticket sync`. Do not hand-edit generated `docs/TICKET_*.md`.
- Docs sync in the same commit as the code — per CLAUDE.md §Documentation. The doc-owner split
  applies: if Cursor owns the doc pass for this program, return verify output rather than writing
  the docs.
- Build via `hcargo` if running from the host; `CARGO_TARGET_DIR=target-container` from a container
  shell. Never share a target dir across host and container.
- No bash step in this program should exceed ~30s except: the wave gate, `trunk` release builds,
  and `leptos-gates`. Anything else over budget is a wrong script, not a slow machine.

## 7. Out of scope

Not part of this program, do not touch:

- `apps/mod/**` — all three Enfusion addons.
- `packages/tbd-schema/**` and generated `apps/website/api_v2/src/contract/generated/**`.
- The API JSON contract — routes, field names, `{data,total,limit,offset}` envelope, auth tiers.
- `packages/map-assets/**` and the `/map-assets` route.
- Any behavior change. This program moves code and builds one new interface (`frame/`). If a phase
  produces a visible behavior difference, that is a bug in the phase.
