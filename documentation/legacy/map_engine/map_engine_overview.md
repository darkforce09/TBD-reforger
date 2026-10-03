**Status:** live

# Map engine overview

The `map_engine` crate holds everything between the platform's map data and the pixels, and the
editing layer through which the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
drives the [mission](/documentation/glossary/g_to_m.md#mission) document. The mission domain itself
(the model, the payload and game-document compilers, the validator, the document and its authoring
commands) is the [mission crates](/crates/mission/README.md), which the
[API](/documentation/glossary/a_to_f.md#api) and the Mission Creator link directly. This overview
covers the crate's layers, the feature tiers its consumers take, and the path from a mounted canvas
to a drawn frame. The code READMEs it links hold the exact detail.

## Where it lives

- Code: [`legacy/map_engine/`](/legacy/map_engine/README.md), whose README gives the
  feature table, the commands and the public surface; [`src/`](/legacy/map_engine/src/README.md),
  whose README gives the nine modules and the tier each compiles under.
- Entry:
  - the Mission Creator installs the editing host (`editing::host::install` in
    `apps/frontend/src/workspaces/editor/mission_editor/canvas_mount.rs`), creates the
    render engine (`RenderEngine::create` in `canvas_mount/boot_tasks.rs`) and starts streaming
    (`streaming::host::bootstrap`, through `apps/frontend/src/workspaces/editor/bridge/world_assets.rs`);
  - the offline tools in `tools/developer_tools/` link the `world` and `streaming` tiers, and
    import the world crates under `crates/` directly for the world export, the
    blueprint tooling and the map checks.
- Related features: [map streaming](/documentation/legacy/map_engine/map_streaming.md), the
  [editing layer](/documentation/legacy/map_engine/editing_layer.md),
  [draft persistence](/documentation/legacy/map_engine/draft_persistence.md), the
  [graphics engine](/documentation/legacy/graphics_engine/graphics_engine_overview.md) it
  draws with, and the [Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md)
  for the app around it.

## Behaviour

### Layers

The modules fall into three sides and a support layer. `world` never names the mission document
(`yrs`, the mission document crates or `editing`), and the mission crates never reach a world
module; the two meet only in `editing`
([engine boundary rules §2D](/documentation/standards/engine_boundary_rules.md#2d-static-world-and-authored-document)).

| Side | Module | What it does | Tier | Deeper doc |
|---|---|---|---|---|
| authored | the mission crates (`crates/mission/`) | the payload compiler, the validator, the Yjs (`yrs`) document the Mission Creator edits and its authoring commands, linked as crates | `editing` | [README](/crates/mission/README.md) |
| authored | `editing` | the editing host, hosted commands, undo drive, draft decisions and map tools | `editing` | [editing layer](/documentation/legacy/map_engine/editing_layer.md) |
| static | `streaming` | fetch, chunk residency, draw buffers and the memory budget | `streaming` | [map streaming](/documentation/legacy/map_engine/map_streaming.md) |
| static | `world_file_formats` (crate) | the binary formats: rkyv archives, containers, density grids, POD layouts | `streaming` | [README](/crates/world_formats/world_file_formats/README.md) |
| static | `world` | the browser loaders, GPU belts and CPU meshes of the terrain and what stands on it, over the terrain and world-object crates | `world` | [README](/legacy/map_engine/src/world/README.md) |
| static | `spatial` | the viewshed lane upload; the BVHs, point indexes, picking and line of sight are the `spatial_indexes` and line of sight crates | `world` | [README](/legacy/map_engine/src/spatial/README.md) |
| draw | `overlay` | the lane preferences and the symbology's GPU bridges; the 48 lanes in paint order and the symbology are the map overlay crates | `world` | [README](/legacy/map_engine/src/overlay/README.md) |
| draw | `frame` | `RenderEngine`, its batch list and upload belts, the frame vocabulary | `world`; GPU half `render` | [README](/legacy/map_engine/src/frame/README.md) |
| draw | `camera` | the render engine's viewport; the orthographic and orbit cameras (`camera_math`) and the grid reference (`map_coordinates`) under map engine paths | always | [README](/legacy/map_engine/src/camera/README.md) |
| support | `diagnostics` | readback checks, the frame benchmark, clocks and console macros | `render` | [README](/legacy/map_engine/src/diagnostics/README.md) |
| support | `doll` | the [arsenal](/documentation/glossary/a_to_f.md#arsenal)'s 3D mannequin preview, a second small renderer | `render` | [README](/legacy/map_engine/src/doll/README.md) |

Browser code (canvas, fetch, image decoding, timers, the console) compiles only for wasm32, most
of it only with `render` as well; everything else builds and tests natively, which is how the
offline tools and the editing layer's tests use the crate.

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
   passes until nothing more arrives. [Map streaming](/documentation/legacy/map_engine/map_streaming.md)
   follows this in detail.
5. Each upload belt puts its lane's batch into the engine's sorted batch list with `upsert_lane`
   and marks the frame damaged; each camera move marks it damaged too. The next `render` builds
   one `FramePacket` over the persistent list and hands it to the graphics engine in one call.
6. Every edit runs through the editing layer and ends in the host's post-change hook, which
   rebinds the affected lanes; the next frame draws them.

The render loop's steps (damage check, camera uniform, surface acquire, compute cull, packet
encode, submit) are in the [frame README](/legacy/map_engine/src/frame/README.md#how-it-works).

### Feature tiers and consumers

A consumer takes the lowest tier that holds what it needs; the map engine README's
[Configuration](/legacy/map_engine/README.md#configuration) lists each feature, what it
turns on and who takes it. No tier is on by default. The chain runs `render → streaming → world`
and `editing → streaming → world`, with `editing` also taking the six mission crates it drives
(`mission_payload`, `mission_validation`, `formation_geometry`, `mission_crdt`,
`mission_document`, `mission_operations`). The graphics engine arrives with `world`, with the
terrain, world-object, map overlay and spatial crates its modules draw; the on-disk formats, the
world format crates and the streamed line of sight arrive with `streaming`. The API links the
mission crates and not this crate, so its tree carries no graphics crate, PNG decoder, `rkyv` or
`flate2`; the crate-tier law keeps the mission crates free of them.

The crate's tests need every feature: `cargo test -p map_engine --all-features`, which
`cargo xtask mk wasm-ci` runs, and without which the tripwire test
`map_engine_tests_require_all_features` fails.

### Known discrepancies

- `CLAUDE.md:159` credits `camera/` with a metric to MGRS unproject; the camera is orthographic
  and orbit arithmetic with pan and zoom controls and a 1000 m grid reference, and the crate has
  no MGRS code (`legacy/map_engine/src/camera/README.md`).
- `CLAUDE.md:162` lists a top-level `symbology/` with NATO MIL-STD-2525 symbols; the symbology is
  the `unit_symbology` crate, uploaded by `overlay/symbology/`, and implements no MIL-STD-2525 set
  (`crates/map_overlay/unit_symbology/README.md`). `CLAUDE.md`'s atlas also omits
  `overlay/` and `frame/`'s role as the render engine's home.

## Data

- `/map-assets/<terrain>/`: the served terrain tree (`assets/terrains/`), which the API mounts;
  [map streaming](/documentation/legacy/map_engine/map_streaming.md#data) lists what the
  loaders read.
- `contracts/rules/kit-aliases.json`: embedded in `mission_payload` at build time.
- The mission payload: the JSON the API stores as a mission version's `json_payload`, which
  `mission_document` hydrates, `mission_payload` and `mission_compiler` compile and
  `mission_validation` validates; the [mission crates README](/crates/mission/README.md) leads to
  each crate's compile and findings.
- The local draft: the document's Yjs encoding, which the Mission Creator keeps in IndexedDB;
  [draft persistence](/documentation/legacy/map_engine/draft_persistence.md) gives the
  decisions the engine makes about it.
- Browser globals the engine publishes for the page and the gates: `window.__mapAssets` (asset
  statistics), `window.__t9386` (the memory ledger), and the render engine's readback checks and
  benchmark, which the Mission Creator publishes as `window.__selfChecks` and
  `window.__editorBench`.

## Design

The crate is one of three layers with a one-way arrow: the frontend uses it, and it uses the pure
renderer, which never names a map concept. The
[engine boundary rules](/documentation/standards/engine_boundary_rules.md) state the layering,
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
  Arland is 4,096 m (`assets/terrains/terrain-registry.json`).

## Open work

- [T-1049 — Add @contract tags to map-engine compiled mission document structs](/.ai/tickets/T-1049.toml)
  (idea, no plan): the AST, world, io and descriptor models that project `contracts` schemas
  gain `@contract` tags the citations gate resolves.
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
- [T-938 — Engine and wasm performance](/documentation/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation/tickets/plans/t-938_plan.md)): pooled lane buffers, measured
  chunk uploads, GPU culling for every icon lane, frame-sliced viewsheds and a wasm memory guard.

## Decisions

- One crate for the world and the editing layer, with walls inside it rather than more crates:
  the walls are cheaper to move than crate boundaries, and the gate enforces them
  ([engine boundary rules §2](/documentation/standards/engine_boundary_rules.md#2-walls-inside-the-map-engine)).
  The mission domain is its own crates, because the API links it with no map code at all.
- No tier is on by default: every consumer names the tiers it links, so no build compiles a GPU,
  image or archive crate it did not ask for.
- The graphics engine arrives with `world`, not `render`: the upload belts import the renderer's
  byte layouts directly, and moving the edge up would mean rewriting them.
- The frontend never depends on the graphics engine; it reaches the render loop through this
  crate's re-export, so the frame vocabulary and the GPU resources have one owner.
