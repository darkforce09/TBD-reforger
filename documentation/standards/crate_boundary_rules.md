**Status:** live

# Crate boundary rules

The crate-level laws of the workspace: which crate may name which, where state lives, how the
map renderer hands frames to the GPU crates, and which law holds each wall. The walls are crate
boundaries: the crate-tier law (`cargo xtask verify crate-tiers`) checks every dependency edge
against its category matrix and its firewalls, and four sibling workspace laws hold the shape of
each crate, its test files, the frontend's layers and the stylesheet's sources. The code is the
final word: the laws live in `tools/foundation/repository_laws/src/workspace_laws/`, and
[section 5](#5-the-workspace-laws) states each rule as that code enforces it, with the file that
enforces it.

## 1. Layers and dependency direction

The web platform draws its map with crates in four categories, each with one job:

| Category | Crates | Job |
|---|---|---|
| `crates/graphics` | [`render_primitives`](/crates/graphics/render_primitives/README.md), [`gpu_device`](/crates/graphics/gpu_device/README.md), [`gpu_frame`](/crates/graphics/gpu_frame/README.md), [`renderer_core`](/crates/graphics/renderer_core/README.md) | the renderer's map-agnostic half: byte layouts, geometry and the WGSL shader, the GPU context of a canvas, the frame vocabulary and its encoder, the pipelines, the animation-frame pump, and the contracts a renderer is built on; it knows no map concept |
| `crates/streaming` | [`chunk_scheduler`](/crates/streaming/chunk_scheduler/README.md), [`chunk_draw_buffers`](/crates/streaming/chunk_draw_buffers/README.md), [`map_streaming_model`](/crates/streaming/map_streaming_model/README.md), [`map_asset_loading`](/crates/streaming/map_asset_loading/README.md), [`map_streaming_host`](/crates/streaming/map_streaming_host/README.md) | the streamed world: chunk residency, the draw buffers, the browser loaders and the map host, which hand the renderer CPU payloads through the `MapAssetSink` contract and never name wgpu |
| `crates/map_rendering` | [`symbology_layers_gpu`](/crates/map_rendering/symbology_layers_gpu/README.md), [`world_layers_gpu`](/crates/map_rendering/world_layers_gpu/README.md), [`map_renderer`](/crates/map_rendering/map_renderer/README.md), [`map_render_diagnostics`](/crates/map_rendering/map_render_diagnostics/README.md) | the map's typed GPU layers and `RenderEngine`, which builds each frame, and the readback self-checks over it |
| `crates/paper_doll` | [`paper_doll_scene`](/crates/paper_doll/paper_doll_scene/README.md), [`paper_doll_renderer`](/crates/paper_doll/paper_doll_renderer/README.md) | the Arsenal's 3D paper doll on its own canvas |

The [mission](/documentation/glossary/g_to_m.md#mission) domain is the crates of `crates/mission/`,
the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s headless editing layer
the crates of `crates/mission_editing/`, and the single-page app ([`frontend`](/apps/frontend/README.md))
links them all. The [API](/documentation/glossary/a_to_f.md#api) (`api`) links none of the map
crates: it compiles and validates missions with the mission crates, and its category edge stops at
foundation, contracts, mission and ballistics crates.

### Dependency direction — non-negotiable

```text
api      ──► crates/mission, crates/ballistics                      native, no map or GPU crate
frontend ──► crates/mission, crates/mission_editing, map_streaming_model   every target
frontend ──► map_renderer, map_streaming_host, map_asset_loading, gpu_frame, …   wasm32
map_rendering, paper_doll ──► streaming, map_overlay, terrain, … (engine) ──► graphics
```

- The arrow runs from the map crates to the graphics category and never back. A graphics crate
  depends on foundation and graphics crates only, and never declares a map noun
  ([section 3](#3-the-graphics-category-a-pure-renderer)).
- The streaming category is an engine category: it depends on foundation, contracts, engine and
  graphics crates and never on a rendering crate, so the loaders and the host reach the renderer
  only through the `MapAssetSink` trait that `map_streaming_model` declares and `map_renderer`
  implements.
- The rendering categories (`crates/map_rendering`, `crates/paper_doll`) may depend on any engine,
  graphics, mission and ballistics crate; nothing below them depends on them.
- Each crate declares `targets = "any"` or `targets = "wasm32"`. A crate built for every target
  reaches a wasm-only crate only from its `[target.'cfg(target_arch = "wasm32")'.dependencies]`
  table, so the native build of the frontend links `map_streaming_model` and none of the GPU or
  browser crates.

### Where state lives

> The engine side (the map crates and the mission and mission editing crates) owns state that
> survives a reload. The frontend owns state that dies with the tab.

| Survives a reload: the engine side | Dies with the tab: `frontend` |
|---|---|
| the mission document, its entities and its undo stack (`crates/mission/mission_document/`, `crates/mission_editing/mission_editing_session/src/history/`) | hover and drag in progress |
| the selection (`crates/mission_editing/mission_editing_session/src/host.rs`) | the pointer gesture state machine (`crates/frontend/workspaces/mission_creator_engine_bridge/src/input/`) |
| the tool command definitions (`crates/mission_editing/mission_editing_commands/`, `crates/mission_editing/map_editing_tools/`) | the keybind map |
| the line-of-sight, ruler and viewshed algorithms (`crates/mission_editing/map_editing_tools/` and the line of sight crates under `crates/line_of_sight/`) | panels open or closed, dock sizes |
| the decisions behind serialising and hydrating a draft (`crates/mission_editing/mission_persistence/`) | the tab lock, the save-status signal and the title (`crates/frontend/workspaces/mission_creator_session/src/`) |

A file whose home is unclear goes where this rule sends it. The frontend side is the Mission
Creator crates under `crates/frontend/workspaces/`. Where the bytes of a draft are stored (IndexedDB) and how
they travel is the frontend's; what they mean is `mission_persistence`'s.

## 2. Walls between the crates

### 2A The mission crates stand outside the map crates

The mission compiler, the validator, the document and its authoring commands are the crates of
`crates/mission/`. The API links them with no map, GPU, image or archive crate in its tree. A
mission crate reaches no world, streaming or graphics code because the category matrix forbids the
edge: a `crates/mission` crate depends on foundation, mission and geometry crates only
(`crate_tiers_a_mission_crate_reaching_world_or_graphics_is_rule_5` pins both edges). A value a
mission crate needs from the world is passed in as an argument.

### 2B The headless editing layer

The crates of `crates/mission_editing/` hold the Mission Creator's decisions: the editing host,
the hosted commands, the undo drive, the draft decisions and the tool state machines. Every one of
them is answerable by `cargo test` with no browser. What only a host can supply (a clock, a frame
pump, a prompt, a storage read) crosses as an injected closure or function pointer, so the layer
names no browser crate. The crate tiers law holds it:

- its firewall refuses a browser crate as a dependency of a mission editing crate, and refuses
  the bare words `web_sys`, `leptos` and `wasm_bindgen` in any `.rs` file under
  `crates/mission_editing/`, in code or prose (`web_sysfs` or `leptosaur` would not match); a
  category folder holding no `.rs` file fails rather than passes;
- its category matrix lets a mission editing crate depend on foundation crates built for every
  target, mission, mission editing, geometry, world format, terrain, world object, line of sight
  and overlay crates only: never a wasm-only foundation crate, a ballistics, streaming, graphics or
  rendering crate.

Inside the layer a comment telling the next reader to reach for `leptos` is the breach the scan
exists to stop. The layer itself is described in the
[editing layer doc](/documentation/crates/mission_editing/editing_layer.md).

### 2C The packet boundary

`map_renderer` builds `gpu_frame`'s `FramePacket` from the world, the document and the overlay, and
it holds `RenderEngine`. Four rules keep the frame path cheap. Breaking any of them costs frame
time, and no functional test notices: the picture stays the same.

1. **Persistent packet.** The packet borrows the engine's persistent batch list
   (`batches: &self.batches` in `crates/map_rendering/map_renderer/src/encode.rs`) and never
   allocates a fresh `Vec<DrawBatch>` a frame. The packet's pipeline and bind-group tables are
   engine fields, cleared and refilled each frame. The one per-frame allocation left is the
   indirect-draw list, built only when the compute cull runs, because an `IndirectDraw` borrows
   buffers inside the engine and cannot be an engine field.
2. **Handles and ranges only.** A `DrawBatch` carries a lane id, a visibility flag, a pipeline id
   and a payload of GPU buffer handles with a stride, count and offset
   (`crates/graphics/gpu_frame/src/frame/batch.rs`, `frame/buffers.rs`), never owned geometry.
   Owned geometry would copy it every frame.
3. **Damage tracking survives.** `render` returns before acquiring a surface texture when
   `RenderDamage` says the frame is clean and not continuous, and every lane mutation marks the
   frame damaged. If packet building became "walk the world, rebuild every batch", the
   damage-driven renderer would turn immediate-mode, and every test would still pass.
4. **One call per frame across the crate boundary.** `encode_main_pass` hands the whole packet to
   `gpu_frame::draw::encode::encode` once a frame, never per batch or per instance: without
   link-time optimisation a chatty boundary is thousands of calls that are not inlined.

Rule 3 is the one whose breach is invisible, so it is pinned in the source:
`crates/map_rendering/map_renderer/src/tests/damage_discipline.rs` reads `lifecycle.rs`,
`encode.rs`, `engine.rs` and `lane_sinks/engine_lane_sink.rs` at compile time and fails if `render`
stops refusing an undamaged frame, a lane mutation stops marking damage, the packet stops reading
the persistent list or the tables are rebuilt rather than refilled. `RenderEngine` needs a GPU
device, so no native test can build one; the source pin is the proof that remains. The damage
state machine itself is tested in `render_primitives`
(`crates/graphics/render_primitives/src/frame/tests/`).

### 2C.1 What may name a GPU type

- **GPU resources.** `wgpu` is a dependency of `crates/map_rendering` crates and of the four GPU
  packages `gpu_device`, `gpu_frame`, `renderer_core` and `paper_doll_renderer` only (the wgpu
  firewall). The map's typed layers receive the device, queue and registries through
  `renderer_core`'s `LayerContext` and write their batches through its `LaneSink`; the GPU context
  of a canvas (create, resize, acquire) is `gpu_device`'s `GpuContext`, which both the map renderer
  and the paper doll renderer adopt.
- **Byte layouts and packing.** The shared binary contract (instance layouts, vertex and geometry
  packing, text metrics and layout, the frame ids, the camera uniform and damage tracking) is
  `render_primitives`, built for every target and holding no GPU handle; any crate may import it
  directly, next to its data.
- **The packet vocabulary.** `FramePacket`, `DrawBatch`, `DrawPayload`, `InstanceBuffer`,
  `TextRun` and the rest of `gpu_frame`'s `frame` module name `wgpu` types, so only the crates the
  wgpu firewall admits can name them.

### 2D Static world and authored document

The streamed world (terrain, world objects, the streaming crates) is immutable, streamed from
`assets/terrains/`, cacheable and never persisted. The mission document (`mission_crdt`,
`mission_document`, `mission_operations`, hosted by the mission editing crates) is mutable,
undoable, CRDT-synced and persisted. They share nothing: a world type never gains a dirty flag, and
a document type never gains a chunk id. The two meet only in the mission editing crates, which host
and drive the document and ask the `spatial_indexes` and line of sight crates where a click or a
sight line lands.

Both sides are crate boundaries. An engine category crate depends on foundation, contracts and
engine crates only, so a world or streaming crate cannot name `yrs`, a mission crate or a mission
editing crate; a mission crate cannot name a chunk id, tile coordinate, level of detail or
residency handle because it cannot depend on the crate that declares it.

## 3. The graphics category: a pure renderer

The graphics crates bind and draw what they are told. They define the GPU frame vocabulary and the
renderer contracts; the map crates decide what to draw, in which order and with which pipeline, and
key every batch on an opaque `LaneId`; the lane roles and their paint order live in
`crates/map_overlay/map_draw_lanes/src/lane_roles.rs`. A map noun in a declared name means a domain
decision crossed the wall: move the decision to a map crate and leave the packing in the graphics
crate. The crate tiers law's graphics firewall refuses a declaration keyword (`struct`, `enum`,
`trait`, `type`, `fn`, `const`, `static`, `mod`) followed by a name containing `terrain`,
`symbology`, `mission`, `orbat` or `arma`, case-insensitive, in any `.rs` file of the category;
prose describing the boundary passes. The
[GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) describes the
crates.

## 4. The frontend: a thin browser app

The frontend owns presentation, input and the browser shell. It mounts `map_renderer`'s
`RenderEngine` on a canvas, drives it with `gpu_frame`'s animation-frame pump, feeds it through
`map_streaming_host`, and never names `wgpu` (the wgpu firewall does not admit it). It supplies what
only a browser has (the canvas, preference readers, clocks, storage) to the map crates as values
and closures, and keeps the state [section 1](#where-state-lives) gives it. The frontend app's start
function is its one JavaScript export.

## 5. The workspace laws

Five laws judge the members of the root `Cargo.toml`. `cargo xtask ci verify-workspace-laws` runs
them in this order and stops at the first failure, and each is also its own `cargo xtask verify`
command: crate tiers, crate anatomy, test-file reachability, frontend layering and Tailwind
sources. Each prints `<LAW>: PASS`, or `FAIL` with exit 1 on a finding and exit 2 when an input
could not be read; a missing input never passes. The paths that move with the tree (the manifest
sweep roots, the frontend layer tables and crate orders, the stylesheet) are constants of
`tools/checks/repository_checks/src/architecture/workspace_law_locations.rs`, which xtask passes
in. Where each law runs is in [CI gates](/documentation/standards/coding_standards/ci_gates.md#verify-workspace-laws)
(rules WS-1 to WS-5).

### 5.1 The judged set

The layout laws judge every member that declares `[package.metadata.layout]` plus every member
under `crates/<category…>/<name>` or `tools/<category>/<name>` (`is_judged` in
`tools/foundation/repository_laws/src/workspace_laws/crate_layout.rs`). Outside that set stand
only the apps under `apps/` and the two tool binaries of `TOOL_BINARY_PATHS`, `tools/xtask` and
`tools/developer_tools`; the report names them in a note. Any other member outside the judged set
is a rule 2 finding: a crate that carries no layout table and sits nowhere the layout knows
cannot slip past the matrix.

### 5.2 Crate tiers (`cargo xtask verify crate-tiers`)

`tools/foundation/repository_laws/src/workspace_laws/crate_tiers.rs` judges seven rules:

1. **Membership.** Every `Cargo.toml` under `apps`, `crates` and `tools` (outside folders named
   `tests`, `fixtures`, `test_fixtures`, `target` and `node_modules`) is a workspace member. The
   root `members` list reaches the library crates through one glob per category
   (`crates/<category>/*`, and `crates/frontend/*/*` for the frontend's layer folders), so a
   manifest in a category no glob lists fails here.
2. **Declarations.** Each judged member declares `[package.metadata.layout]` with a `category`, a
   whole-number `tier` and `targets` (`any` or `wasm32`); every member outside the judged set is
   an app or a tool binary ([5.1](#51-the-judged-set)).
3. **Location.** The declared category is the crate's parent folder and a category the layout
   knows, and the package name equals the folder name.
4. **Tiers.** A crate with no judged workspace dependency is tier 0; any other is 1 plus the
   highest tier among its dependencies, over normal and build edges in every target table. The
   declared tier equals that number, and every edge points strictly down.
5. **Category matrix.** Every normal and build edge is allowed by the matrix of
   [5.3](#53-the-category-matrix), and a crate built for every target reaches a `wasm32` crate
   only from its `[target.'cfg(target_arch = "wasm32")'.dependencies]` table.
6. **Firewalls.** The external-crate and source firewalls of [5.4](#54-the-firewalls) hold.
7. **Dev-dependencies.** A dev-dependency is outside the tier order and the matrix, because it
   never ships, but it never points at an app under `apps/`.

### 5.3 The category matrix

`category_edge_allowed` in `crate_layout.rs` decides rule 5. A category's class comes from its
folder path alone, and an edge to or from a category with no class (an unknown category, an app,
a tool binary) is refused.

| A crate in | May depend on |
|---|---|
| `crates/foundation` | foundation crates |
| `crates/contracts` | foundation and contracts crates |
| `crates/mission` | foundation, mission and `crates/geometry` crates |
| `crates/ballistics` | foundation and ballistics crates |
| `crates/graphics` | foundation and graphics crates |
| the other engine categories: `crates/geometry`, `crates/world_formats`, `crates/terrain`, `crates/world_objects`, `crates/line_of_sight`, `crates/map_overlay`, `crates/streaming` | foundation and contracts crates and every engine category, graphics included |
| `crates/map_rendering`, `crates/paper_doll` | foundation, contracts, mission, ballistics and rendering crates and every engine category |
| `crates/mission_editing` | foundation crates whose `targets` is not `wasm32`; mission and mission editing crates; the geometry, world formats, terrain, world objects, line of sight and map overlay categories (never ballistics, streaming, graphics or rendering) |
| `crates/api` | foundation, contracts, mission, ballistics and api crates |
| `crates/frontend/<layer>` (`foundation`, `features`, `pages`, `workspaces`) | every class but api and tools |
| `tools/<category>` | foundation, contracts, mission, ballistics and tool crates, and engine crates whose `targets` is `any`; never a `wasm32` crate; the api crates from `tools/staging/staging_fixtures` only (`STAGING_FIXTURES_PATH`) |

The engine categories are `ENGINE_CATEGORIES` (the graphics category among them), and the mission
editing subset is `MISSION_EDITING_ENGINE_CATEGORIES`. Inside the frontend class the
[frontend-layering law](#57-frontend-layering-cargo-xtask-verify-frontend-layering) orders the
layers.

### 5.4 The firewalls

`tools/foundation/repository_laws/src/workspace_laws/crate_firewalls.rs` judges rule 6. The
manifest firewalls read the normal and build edges of each judged member onto an external crate
(a package no workspace member carries):

- **wgpu** (and `wgpu-*`): only in `crates/map_rendering` crates and the packages of
  `GPU_PACKAGES`: `gpu_device`, `gpu_frame`, `renderer_core` and `paper_doll_renderer`.
- **Browser crates** (`web-sys`, `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures`, `gloo`,
  `gloo-*`): never in a mission editing crate; elsewhere only in `wasm32` crates, frontend crates,
  and `time_source` from a target table.
- **sqlx and axum** (and `sqlx-*`, `axum-*`): only in api crates. Axum is also allowed in the
  categories of `HARNESS_SERVER_CATEGORIES`, `tools/browser_testing` and `tools/staging`, whose
  crates are test and staging harness servers rather than product code; sqlx is also allowed in
  the one crate at `SQLX_STAGING_TOOL_PATH`, `tools/staging/staging_fixtures`, the staging host
  tool that seeds and cleans staging rows directly.
- **leptos** (and `leptos_*`, `leptos-*`): only in frontend crates.
- **The xtask closure:** no member of the dependency closure of xtask, walked from the xtask
  member over normal and build edges although xtask is outside the judged set, declares
  `tokio`, `axum`, `reqwest`, `resvg` or `image` (`XTASK_CLOSURE_BANS`); this keeps the harness
  servers out of xtask.

Three source scans complete the rule:

- **Map nouns in graphics:** no line of a graphics crate's sources matches
  `MAP_NOUN_DECLARATION_PATTERN`, a declaration keyword (`struct`, `enum`, `trait`, `type`, `fn`,
  `const`, `static`, `mod`) followed by an identifier holding `terrain`, `symbology`, `mission`,
  `orbat` or `arma`, case-insensitive ([section 3](#3-the-graphics-category-a-pure-renderer)).
- **Browser words in mission editing:** no `.rs` file under `crates/mission_editing/` spells
  `web_sys`, `leptos` or `wasm_bindgen` as a whole word, in code or prose
  ([2B](#2b-the-headless-editing-layer)); a category folder holding no `.rs` file is a finding,
  never a clean scan.
- **JavaScript exports:** no `.rs` file of a workspace member outside `apps/frontend`,
  `crates/foundation/browser_platform` and `apps/offline_service_worker`
  (`WASM_BINDGEN_EXPORT_FOLDERS`) carries a `#[wasm_bindgen]` attribute, plain, path-qualified
  or under `cfg_attr`, at the start of a line; a walk of no `.rs` file at all is a finding.

### 5.5 Crate anatomy (`cargo xtask verify crate-anatomy`)

`crate_anatomy.rs` and `crate_anatomy_sources.rs` hold every judged library crate (a crate with
`[lib]` or `src/lib.rs`; a crate of binaries only is exempt and named in a note) to one shape:

- `lib.rs` is at most 80 lines of doc comments, attributes, `mod` and `pub use` lines, and
  declares `pub mod prelude`;
- a `pub fn` returning `Result` needs an `error.rs` with a `thiserror` `pub enum Error` and a
  `pub type Result`; no `anyhow` dependency;
- a README.md with a Contents block;
- `edition`, `rust-version`, `[lints]` and every dependency come from the workspace;
- the only features are `test_fixtures` and `failpoints`, each enabled only by a dev-dependency;
- no primitive-typed public `id` or `*_id` field or `pub fn` parameter, outside `generated`
  folders and `#[wasm_bindgen]` items;
- no public re-export of another workspace crate outside the crate's prelude module, in any form
  `public_reexports.rs` reads (an item, a module or the crate root, an alias, a group, a statement
  over several lines, `pub extern crate`). Test files are never judged.

### 5.6 Test-file reachability (`cargo xtask verify test-file-reachability`)

`test_file_reachability.rs` finds every `.rs` file of a member that sits in a `tests` folder,
under `src/` or in the member's own `tests/` folder, and that no target of the member loads. The
walk starts at the target roots (the manifest's `[lib]` and `[[bin]]` paths, `src/lib.rs`,
`src/main.rs`, `src/bin/`, `tests/*.rs`, `tests/*/main.rs`, `benches/*.rs`, `examples/*.rs`) and
follows every `mod` declaration with its `#[path]`, plus every trybuild case
(`compile_fail` / `pass`) a loaded file names. An unreached file is a finding: its tests never
run while the tree looks covered. An integration target is never itself a finding, and the walk
skips `target` folders and any folder holding its own `Cargo.toml`.

### 5.7 Frontend layering (`cargo xtask verify frontend-layering`)

The frontend's layers are foundation < features < pages and workspaces < the app shell
(`apps/frontend`). Pages and workspaces are peers: neither depends on the other, and no page crate
depends on another page crate. `frontend_layering.rs` and `frontend_layering/crate_edges.rs` judge
two modes, hard at zero, so every edge (production or test, normal, dev or build) fails:

- **crate-edge mode** (`FRONTEND_CRATE_EDGES`): a crate's layer is its folder
  `crates/frontend/<layer>/`, and the crate orders hold inside a layer folder:
  `FOUNDATION_CRATE_ORDER` (`frontend_ui` < `frontend_api_dtos` < {`frontend_transport`,
  `frontend_route_table`} < `frontend_session` < {`frontend_offline`, `frontend_map_view`}, the
  braced crates peers, `frontend_test_support` reached only through dev-dependencies),
  `MISSION_CREATOR_CRATE_ORDER` (`mission_creator_state` < `mission_creator_engine_bridge` <
  `mission_creator_session` < `mission_creator_arsenal` < `mission_creator_workspace`) and
  `DEBUG_BENCHES_CRATE_ORDER`, an order of its own that no Mission Creator crate touches. A
  frontend crate in no layer folder or in none of its folder's orders, and a crate an order names
  that no member carries, are findings;
- **in-crate mode** (`APP_LAYERS`): the app's own modules (`main.rs`, `app_routes.rs`, `shell/`,
  `tests/`) all map onto the shell layer, and a source no row maps is a finding.

### 5.8 Tailwind sources (`cargo xtask verify tailwind-sources`)

`tailwind_sources.rs` holds the `@source` lines of `apps/frontend/style/aegis.css` exact: every
member with a non-dev `leptos` dependency, the app included, is named by exactly one line whose
glob, resolved from the stylesheet's folder, is `<member>/src/**/*.rs`. A member no line names, a
member several lines name, and a line naming no leptos member (an ancestor or wildcard folder
included) are findings. Trunk's `[watch]` list in `apps/frontend/Trunk.toml` covers
`crates/frontend`, so a change in any frontend crate rebuilds the bundle.

### 5.9 The apps and the tool crates

The apps carry no layout table, so the matrix does not judge them. Their limits are deny-lists in
`tools/foundation/repository_laws/src/crate_dependencies.rs`, run by the `engineering_laws` test
binary of `apps/api`: the frontend never links `api`; `api` links neither `frontend` nor a GPU
crate; the offline service worker links none of `api`, `frontend` or a GPU crate.

The tool crates keep a direction of their own, pinned by
`tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`
(`cargo test -p repository_checks`):

- the tool binaries `tools/xtask` and `tools/developer_tools` hold no library, depend among the
  workspace crates only on tool crates at `tools/<category>/<name>`, never on each other, and no
  member depends on either; xtask reaches the checkout root through `repository_layout`'s
  prelude;
- a `tools/foundation` crate depends only on `tools/foundation` crates of a lower declared tier
  and on `crates/foundation/repository_root`, the workspace's one checkout-root finder;
- a `tools/tickets` crate depends only on `tools/foundation` crates, on `time_source`,
  `content_digest`, `newtype_ids` and `repository_root`, and on `tools/tickets` crates of a lower
  declared tier.

### 5.10 Which law holds which wall

| Wall | Law | Where |
|---|---|---|
| Edges point down the tiers and across the allowed categories | crate tiers rules 4 and 5 | `crate_tiers.rs`, the matrix in `crate_layout.rs` |
| wgpu only in the rendering category and the GPU packages | crate tiers firewall | `crate_firewalls.rs` |
| Browser crates only in wasm-only crates, `time_source` behind a target table and the frontend | crate tiers firewall | `crate_firewalls.rs` |
| A wasm-only crate reached from a crate built for every target only through its wasm32 table | crate tiers rule 5 | `crate_tiers.rs` |
| No map noun in a graphics declaration | crate tiers firewall | `crate_firewalls.rs` |
| No browser crate or token in the mission editing layer | crate tiers firewall | `crate_firewalls.rs` |
| Every library crate keeps the standard shape | crate anatomy | `crate_anatomy.rs`, `crate_anatomy_sources.rs` |
| Every test file compiles | test-file reachability | `test_file_reachability.rs` |
| Lower frontend layers never name higher ones; pages and workspaces stay apart | frontend layering | `frontend_layering.rs` |
| Every leptos crate's classes reach the stylesheet | Tailwind sources | `tailwind_sources.rs` |
| The frame path stays damage-driven | source pins | `crates/map_rendering/map_renderer/src/tests/damage_discipline.rs` |

Every law runs in `cargo xtask ci ci-local` and in CI; a law whose input is missing fails rather
than passes.

## 6. Known gaps and open work

- **The map noun list is short.** Declared names such as `BuildingInstance` and the WGSL entry
  points `vs_building`, `fs_building` and `fs_forest_density` name map concepts the five nouns do
  not catch (`crates/graphics/render_primitives/src/draw/instances.rs`,
  `crates/graphics/render_primitives/src/shaders/shader.wgsl`).

Tickets:

- [T-1070 — Remove map nouns from graphics engine names; widen rule 2](/.ai/tickets/T-1070.toml)
  (idea, no plan): renames the graphics crates' map-named items to geometry terms and adds
  building, forest, map, hillshade, slot, town and road to the noun list.

## Related documentation

- [Map rendering documentation](/documentation/crates/map_rendering/README.md) — the map renderer
  and its typed layers.
- [Map streaming documentation](/documentation/crates/streaming/README.md) — the loaders, the map
  host and the asset sink contract.
- [GPU rendering documentation](/documentation/crates/graphics/README.md) — the graphics crates.
- [Workspace laws README](/tools/foundation/repository_laws/src/workspace_laws/README.md) — the
  code of the five workspace laws and their tests.
- [CI gates](/documentation/standards/coding_standards/ci_gates.md#verify-workspace-laws) — where
  the workspace laws run (rules WS-1 to WS-5).
- [Coding standards](/documentation/standards/coding_standards/README.md) — the repository's
  other code rules.
