**Status:** live

# Map engine overview

The `website-map-engine` crate holds everything between the platform's map data and the pixels,
and the [mission](/documentation_v2/glossary/g_to_m.md#mission) domain that the
[API](/documentation_v2/glossary/a_to_f.md#api) and the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) share. This overview covers its
layers, the feature tiers its consumers take, and the path from a mounted canvas to a drawn frame.
The code READMEs it links hold the exact detail.

## Where it lives

- Code: [`apps/website/map-engine/`](/apps/website/map-engine/README.md), whose README gives the
  feature table, the commands and the public surface; [`src/`](/apps/website/map-engine/src/README.md),
  whose README gives the eleven modules and the tier each compiles under.
- Entry:
  - the API links the default `scenario` tier (`apps/website/api_v2/Cargo.toml:65`) and calls
    `data::scenario` from its missions and operations domains;
  - the Mission Creator installs the editing host (`editing::host::install` in
    `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`), creates the
    render engine (`RenderEngine::create` in `canvas_mount/boot_tasks.rs`) and starts streaming
    (`streaming::host::bootstrap`, through `apps/website/frontend/src/v2/apps/editor/bridge/world_assets.rs`);
  - the offline tools in `tools_v2/developer-tools/` link the `world`, `streaming`, `io` and `bvh`
    tiers for the world export, the blueprint tooling and the map checks.
- Related features: [map streaming](/documentation_v2/website/map-engine/map_streaming.md), the
  [editing layer](/documentation_v2/website/map-engine/editing_layer.md),
  [draft persistence](/documentation_v2/website/map-engine/draft_persistence.md), the
  [graphics engine](/documentation_v2/website/graphics-engine/graphics_engine_overview.md) it
  draws with, and the [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md)
  for the app around it.

## Behaviour

### Layers

The modules fall into three sides and a support layer. `data` never names the world, and `world`
never names the document; they meet only in `editing`
([engine boundary rules §2D](/documentation_v2/standards/engine_boundary_rules.md#2d-static-world-and-authored-document)).

| Side | Module | What it does | Tier | Deeper doc |
|---|---|---|---|---|
| authored | `data/scenario` | the mission compiler, validator and AST | `scenario` | [README](/apps/website/map-engine/src/data/scenario/README.md) |
| authored | `data/store` | the Yjs (`yrs`) document the Mission Creator edits, its rows and operations | `store` | [README](/apps/website/map-engine/src/data/store/README.md) |
| authored | `editing` | the editing host, hosted commands, undo drive, draft decisions and map tools | `editing` | [editing layer](/documentation_v2/website/map-engine/editing_layer.md) |
| static | `streaming` | fetch, chunk residency, draw buffers and the memory budget | `io`, `streaming` | [map streaming](/documentation_v2/website/map-engine/map_streaming.md) |
| static | `io` | the binary formats: rkyv archives, containers, density grids, POD layouts | `io` | [README](/apps/website/map-engine/src/io/README.md) |
| static | `world` | terrain, what stands on it, and building interiors | `world` | [README](/apps/website/map-engine/src/world/README.md) |
| static | `spatial` | BVHs, point indexes, picking and line of sight | `world` | [README](/apps/website/map-engine/src/spatial/README.md) |
| draw | `overlay` | the 48 lanes in paint order and the symbology drawn in them | `world` | [README](/apps/website/map-engine/src/overlay/README.md) |
| draw | `frame` | `RenderEngine`, its batch list and upload belts, the frame vocabulary | `world`; GPU half `render` | [README](/apps/website/map-engine/src/frame/README.md) |
| draw | `camera` | the map's orthographic camera, the doll's orbit camera, the grid reference | always | [README](/apps/website/map-engine/src/camera/README.md) |
| support | `diagnostics` | readback checks, the frame benchmark, clocks and console macros | `render` | [README](/apps/website/map-engine/src/diagnostics/README.md) |
| support | `doll` | the [arsenal](/documentation_v2/glossary/a_to_f.md#arsenal)'s 3D mannequin preview, a second small renderer | `render` | [README](/apps/website/map-engine/src/doll/README.md) |

Browser code (canvas, fetch, image decoding, timers, the console) compiles only for wasm32, most
of it only with `render` as well; everything else builds and tests natively, which is how the API
and the offline tools use the crate.

### From a mounted canvas to a drawn frame

1. The Mission Creator creates the mission document and the selection, and installs them as the
   editing host (`editing::host::install`), so every hosted command and undo sees the live
   document.
2. It creates the render engine with `RenderEngine::create(canvas, force_webgl)`: WebGPU with a
   WebGL2 fallback, or WebGL2 alone when forced. On success it sizes the surface, bounds and
   places the camera, hides the calibration quads, turns continuous rendering off (the engine is
   damage-driven), uploads the slot icon atlas, registers the render context, and binds the
   document's slots and vehicles to their symbology lanes.
3. It starts the graphics engine's `RafPump` on the engine, reached through the map engine's
   re-export (`crate::frame::RafPump`); each animation frame calls `render`, which returns at once
   unless something marked the frame damaged.
4. It starts streaming for the terrain the document names (`meta.terrain`, else `everon`):
   `streaming::host::bootstrap` fetches the terrain manifest, loads the elevation model, hillshade
   and satellite basemap, then the world objects, forest, water and labels, and runs viewport
   passes until nothing more arrives. [Map streaming](/documentation_v2/website/map-engine/map_streaming.md)
   follows this in detail.
5. Each upload belt puts its lane's batch into the engine's sorted batch list with `upsert_lane`
   and marks the frame damaged; each camera move marks it damaged too. The next `render` builds
   one `FramePacket` over the persistent list and hands it to the graphics engine in one call.
6. Every edit runs through the editing layer and ends in the host's post-change hook, which
   rebinds the affected lanes; the next frame draws them.

The render loop's steps (damage check, camera uniform, surface acquire, compute cull, packet
encode, submit) are in the [frame README](/apps/website/map-engine/src/frame/README.md#how-it-works).

### Feature tiers and consumers

A consumer takes the lowest tier that holds what it needs; the map engine README's
[Configuration](/apps/website/map-engine/README.md#configuration) lists each feature, what it
turns on and who takes it. The chain runs `render → streaming → io → world → bvh`, with `io`
also taking `scenario`, and `editing → store → scenario` with `editing` also taking `world` and
`streaming`. The graphics engine arrives with `world`. The API's `scenario` tier carries no
graphics crate, PNG decoder, `rkyv` or `flate2`; the gate's rule 4 keeps it so.

The crate's tests need every feature: `cargo test -p website-map-engine --all-features`, which
`cargo xtask mk wasm-ci` runs, and without which the tripwire test
`map_engine_tests_require_all_features` fails.

### Known discrepancies

- `CLAUDE.md:159` credits `camera/` with a metric to MGRS unproject; the camera is orthographic
  and orbit arithmetic with pan and zoom controls and a 1000 m grid reference, and the crate has
  no MGRS code (`apps/website/map-engine/src/camera/README.md`).
- `CLAUDE.md:162` lists a top-level `symbology/` with NATO MIL-STD-2525 symbols; the symbology is
  `overlay/symbology/` and implements no MIL-STD-2525 set
  (`apps/website/map-engine/src/overlay/symbology/README.md`). `CLAUDE.md`'s atlas also omits
  `overlay/` and `frame/`'s role as the render engine's home.
- `apps/website/map-engine/Cargo.toml:24-25` explains the `world` tier's graphics edge with
  `frontend/Cargo.toml:27` and `renderers::primitives::triangulate`; the frontend declares the
  map engine at `apps/website/frontend/Cargo.toml:33`, and the path is
  `website_map_engine::world::mesh::triangulate` (`src/world/mesh.rs:17`).
- `apps/website/map-engine/src/lib.rs:1-4` calls the crate root a module "in the graphics engine"
  with placeholder role lines; the crate is the map engine.

## Data

- `/map-assets/<terrain>/`: the served terrain tree (`assets_v2/terrains/`), which the API mounts;
  [map streaming](/documentation_v2/website/map-engine/map_streaming.md#data) lists what the
  loaders read.
- `contracts_v2/rules/kit-aliases.json`: embedded in `data::scenario` at build time.
- The mission payload: the JSON the API stores as a mission version's `json_payload`, which
  `data::store` hydrates and `data::scenario` compiles and validates; the
  [scenario README](/apps/website/map-engine/src/data/scenario/README.md) gives the compile and
  its findings.
- The local draft: the document's Yjs encoding, which the Mission Creator keeps in IndexedDB;
  [draft persistence](/documentation_v2/website/map-engine/draft_persistence.md) gives the
  decisions the engine makes about it.
- Browser globals the engine publishes for the page and the gates: `window.__mapAssets` (asset
  statistics), `window.__t9386` (the memory ledger), and the render engine's readback checks and
  benchmark, which the Mission Creator publishes as `window.__selfChecks` and
  `window.__editorBench`.

## Design

The crate is one of three layers with a one-way arrow: the frontend uses it, and it uses the pure
renderer, which never names a map concept. The
[engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) state the layering,
the rule that decides where state lives, the four frame-path rules and the walls inside this
crate; `cargo xtask verify engine-layers` holds them.

The design target is the layout those rules describe. Where the built crate differs:

- `RenderEngine`, which owns every GPU resource the renderer draws with, lives in this crate's
  `frame/`, not in the graphics engine, so five sites still name the graphics engine's `device`,
  `pipeline` and `r#loop` modules, each pinned by the gate (rule 3b).
- The browser ban covers `editing/` alone; the streaming host, the readback diagnostics, the doll
  renderer and the render engine reach the browser on purpose.
- Several map constants are fixed to Everon's 12,800 m square: the grid, the basemap extent, the
  contours, the peaks and the forest density grid, and the Mission Creator's camera bounds, while
  Arland is 4,096 m (`assets_v2/terrains/terrain-registry.json`).

## Open work

- [T-1049 — Add @contract tags to map-engine compiled mission document structs](/.ai/tickets/T-1049.toml)
  (idea, no plan): the AST, world, io and descriptor models that project `contracts_v2` schemas
  gain `@contract` tags the citations gate resolves.
- [T-1055 — Fix engine-layers gate omitting the map engine editing module](/.ai/tickets/T-1055.toml)
  (idea, no plan): `data/` and `data/scenario/` naming `crate::editing` starts failing the gate.
- [T-1058 — Fix map basemap switch back never restoring the satellite imagery](/.ai/tickets/T-1058.toml)
  (idea, no plan): switching the basemap from `map` back to `satellite` shows the imagery again.
- [T-1059 — Check whether empty uploads leave stale height and road labels](/.ai/tickets/T-1059.toml)
  (idea, no plan): an empty height or road label upload removes the old labels, as the town
  labels do.
- [T-1061 — Check world line-of-sight sidecar eviction ignoring recency](/.ai/tickets/T-1061.toml)
  (idea, no plan): the line-of-sight occluder's sidecar cap evicts the least recently used
  sidecar, as the chunk residency does, instead of the lowest path, since its use tick never
  advances.
- [T-1062 — Derive map grid, basemap, peaks and forest from terrain size](/.ai/tickets/T-1062.toml)
  (idea, no plan): the fixed 12,800 m constants give way to the loaded terrain's size.
- [T-1063 — Check map render bench stress helpers and calibration hide](/.ai/tickets/T-1063.toml)
  (idea, no plan): `seed_stress` and `clear_stress` stop destroying the map lanes, or go.
- [T-1064 — Add guards for map binary formats and doll shader layout](/.ai/tickets/T-1064.toml)
  (idea, no plan): the density decoder checks its version, and the doll's region and layout
  constants get tests.
- [T-1065 — Lint map engine render code in the wasm32 clippy step](/.ai/tickets/T-1065.toml)
  (idea, no plan): `wasm-ci`'s wasm32 clippy compiles the render tiers, not the default tier alone.
- [T-1066 — Fix map object instance schema naming a nonexistent pod file](/.ai/tickets/T-1066.toml)
  (idea, no plan): the schema cites `io/pod/instance.rs`.
- [T-1067 — Remove dead map engine code, facades and duplicated constants](/.ai/tickets/T-1067.toml),
  [T-1068 — Rewrite stale map engine comments outside mission data](/.ai/tickets/T-1068.toml) and
  [T-1069 — Rename unclear map engine modules and numbered file splits](/.ai/tickets/T-1069.toml)
  (idea, no plan): cleanup across the crate, including the placeholder module headers.
- [T-1071 — Decide whether ignored map engine inputs are intended](/.ai/tickets/T-1071.toml)
  (idea, no plan): the hillshade slope scale, the satellite `Retry-After` and the archived
  blueprints' door and furniture fields are used or documented as ignored.
- [T-1042 — Rename ticket ids out of code names and UI strings](/.ai/tickets/T-1042.toml) (idea,
  no plan): the 16 map-engine test folders and files named after tickets get subject names.
- [T-938 — Engine and wasm performance](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-938_plan.md)): pooled lane buffers, measured
  chunk uploads, GPU culling for every icon lane, frame-sliced viewsheds and a wasm memory guard.

## Decisions

- One crate for the data, the world and the editing layer, with walls inside it rather than more
  crates: the walls are cheaper to move than crate boundaries, and the gate enforces them
  ([engine boundary rules §2](/documentation_v2/standards/engine_boundary_rules.md#2-walls-inside-the-map-engine)).
- The default tier is `scenario`: the API links the compiler and nothing else, so an HTTP server
  never compiles a GPU, image or archive crate.
- The graphics engine arrives with `world`, not `render`: the upload belts import the renderer's
  byte layouts directly, and moving the edge up would mean rewriting them.
- The frontend never depends on the graphics engine; it reaches the render loop through this
  crate's re-export, so the frame vocabulary and the GPU resources have one owner.
