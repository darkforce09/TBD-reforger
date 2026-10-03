**Status:** live

# Engine boundary rules

The layer rules between the website's two engine crates and the app that uses them: which crate
may name which, where state lives, how the map engine hands frames to the renderer, and the walls
inside the map engine. `cargo xtask verify engine-layers` enforces the rules section 5 numbers,
and its failure messages, the CI step and the `ci-local` task cite this document by section. The
code is the final word: the gate's matchers and pins in
`tools/foundation/repository_laws/src/engine_layers/` define each rule exactly, and this document
states them and the reasons behind them.

## 1. Layers and dependency direction

The web platform draws its map with three crates, each with one job:

| Crate | Code | Job |
|---|---|---|
| `graphics_engine` | [`legacy/graphics_engine/`](/legacy/graphics_engine/README.md) | the renderer: device buffers, pipelines, the WGSL shader, draw batching, text packing, sprite culling and the animation-frame pump; it knows no map concept |
| `map_engine` | [`legacy/map_engine/`](/legacy/map_engine/README.md) | the editing layer over the [mission](/documentation/glossary/g_to_m.md#mission) crates, the static world's loaders and belts, streaming, the viewshed and symbology uploads, the camera viewport, the headless editing layer and `RenderEngine`, which builds each frame; the world's data models, spatial queries and the overlay's lanes and symbology are crates under `crates/` it imports |
| `frontend` | [`apps/frontend/`](/apps/frontend/README.md) | the single-page app, including the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): pages, UI, input and the browser shell |

The [API](/documentation/glossary/a_to_f.md#api) (`api`) links none of the three: it compiles and
validates missions with the mission crates of `crates/mission/`, and `API_RULE` of
`tools/foundation/repository_laws/src/crate_dependencies.rs` forbids its edge to the map engine.

### Dependency direction — non-negotiable

```text
api      ──► crates/mission, crates/ballistics       native, no map engine, no GPU crate
frontend ──► map_engine {world, editing}             every target
                                        {render, streaming}          wasm32
                     map_engine ──► graphics_engine  from the world tier up
                     map_engine ──► crates/mission   the editing tier
```

- The arrow runs from the map engine to the graphics layer and never back. The graphics layer is
  the graphics engine and every member of the `crates/graphics` category (today
  [`render_primitives`](/crates/graphics/render_primitives/README.md), the GPU-free building
  blocks the graphics engine is built on). It never learns a map noun; the map engine speaks the
  renderer's frame vocabulary, not the reverse
  ([rule 1](#rule-1-the-graphics-layer-never-imports-the-map-engine),
  [rule 2](#rule-2-no-map-noun-in-a-graphics-layer-declaration)).
- The frontend never depends on the graphics engine or on a wasm-only graphics crate. It hands
  the map engine a canvas, and reaches the render loop through the map engine's re-export of
  `RafPump` and `FrameTarget` (`legacy/map_engine/src/frame/mod.rs`, `frame/pump.rs`)
  ([rule 6](#rule-6-the-frontend-never-imports-the-gpu-layer)). A graphics crate built for every
  target (`targets = "any"`) is CPU code the frontend may link.
- Nothing in the compiler enforces either direction: a dependency edge pointing back would still
  build. The gate is the only wall.

The graphics engine's own layout and public surface are in its
[README](/legacy/graphics_engine/README.md); the map engine's modules and feature tiers are
in its [README](/legacy/map_engine/README.md) and [source README](/legacy/map_engine/src/README.md).

### Where state lives

> The map engine owns state that survives a reload. The frontend owns state that dies with the
> tab.

| Survives a reload: `map_engine` | Dies with the tab: `frontend` |
|---|---|
| the mission document, its entities and its undo stack (`crates/mission/mission_document/`, `editing/history/`) | hover and drag in progress |
| the selection (`editing/host.rs`) | the pointer gesture state machine (`editor/input/`) |
| the tool command definitions (`editing/commands/`, `editing/tools/`) | the keybind map |
| the line-of-sight, ruler and viewshed algorithms (`editing/tools/` and the line of sight crates under `crates/line_of_sight/`) | panels open or closed, dock sizes |
| the decisions behind serialising and hydrating a draft (`editing/persist/`) | the tab lock, the save-status signal and the title (`editor/shell/`) |

A file whose home is unclear goes where this rule sends it. The frontend paths are under
`apps/frontend/src/workspaces/editor/`; the map engine paths under
`legacy/map_engine/src/`. Where the bytes of a draft are stored (IndexedDB) and how they
travel is the frontend's; what they mean is the map engine's.

### Feature tiers

A consumer takes only the tier it needs; the feature table and its consumers are in the map
engine README's [Configuration](/legacy/map_engine/README.md#configuration). Three facts of
the tiers are boundary rules:

- No tier is on by default, and the mission domain is no tier at all: it is the crates of
  `crates/mission/`, which the API links directly and the `editing` tier links for the Mission
  Creator ([2A](#2a-the-mission-crates-stand-outside-the-map-engine)).
- `world` turns the graphics engine and `render_primitives` on, not `render` alone
  (`legacy/map_engine/Cargo.toml`): the upload belts of `world/`, `overlay/` and `spatial/`
  import the renderer's byte layouts and geometry helpers directly (Kind B in
  [2C.1](#2c1-what-may-name-a-graphics-type)), and `world/mesh.rs` composes meshes with
  `render_primitives::draw::{compose, triangulate}`.
- `editing` takes `world`, `streaming` and the six mission crates it hosts and drives, because
  the line-of-sight tool tests cells against the streamed world's occluder, the
  `world_line_of_sight` crate `streaming` links (`legacy/map_engine/Cargo.toml`).

## 2. Walls inside the map engine

The map engine is one crate of nine top-level modules (`legacy/map_engine/src/lib.rs`).
Three walls inside it, and one crate boundary beside it, replace the crate boundaries a split into
more crates would have drawn.

### 2A The mission crates stand outside the map engine

The mission compiler, the validator, the document and its authoring commands are the crates of
`crates/mission/`, not modules of the map engine. The API links them with no map engine, GPU,
image or archive crate in its tree; the map engine's `editing` tier links the ones it drives. A
mission crate reaches no world, streaming or graphics code because the crate-tier law forbids the
edge: a `crates/mission` crate depends on foundation, mission and geometry crates only and on
nothing under `legacy/` (`cargo xtask verify crate-tiers`;
`crate_tiers_a_mission_crate_reaching_world_or_graphics_is_rule_5` pins both edges). A value a
mission crate needs from the world is passed in as an argument. No engine-layer rule repeats the
edge, so section 5 has no rule 4.

### 2B The headless editing layer

`editing/` holds the Mission Creator's decisions: the editing host, the hosted commands, the undo
drive, the draft decisions and the tool state machines. Every one of them is answerable by
`cargo test` with no browser. What only a host can supply (a clock, a frame pump, a prompt, a
storage read) crosses as an injected closure or function pointer, so the tree names no browser
crate. [Rule 5](#rule-5-no-browser-crate-in-the-editing-layer) enforces it; the layer itself is
described in the [editing layer doc](/documentation/legacy/map_engine/editing_layer.md).

The rest of the crate is a wasm crate that reaches the browser on purpose (the streaming host,
the readback diagnostics, the doll renderer, the render engine), so the browser ban covers
`editing/` alone.

### 2C The packet boundary

`legacy/map_engine/src/frame/` builds the graphics engine's `FramePacket` from the world,
the document and the overlay, and it holds `RenderEngine`. Four rules keep the frame path cheap.
Breaking any of them costs frame time, and no functional test notices: the picture stays the
same.

1. **Persistent packet.** The packet borrows the engine's persistent batch list
   (`batches: &self.batches` in `frame/encode.rs`) and never allocates a fresh `Vec<DrawBatch>` a
   frame. The packet's pipeline and bind-group tables are engine fields, cleared and refilled each
   frame. The one per-frame allocation left is the indirect-draw list, built only when the compute
   cull runs, because an `IndirectDraw` borrows buffers inside the engine and cannot be an engine
   field (`frame/encode.rs:168-177`).
2. **Handles and ranges only.** A `DrawBatch` carries a lane id, a visibility flag, a pipeline id
   and a payload of GPU buffer handles with a stride, count and offset
   (`legacy/graphics_engine/src/frame/batch.rs`, `frame/buffers.rs`), never owned geometry.
   Owned geometry would copy it every frame.
3. **Damage tracking survives.** `render` returns before acquiring a surface texture when
   `RenderDamage` says the frame is clean and not continuous, and every lane mutation marks the
   frame damaged. If packet building became "walk the world, rebuild every batch", the
   damage-driven renderer would turn immediate-mode, and every test would still pass.
4. **One call per frame across the crate boundary.** `encode_main_pass` hands the whole packet to
   `graphics_engine::draw::encode::encode` once a frame, never per batch or per instance:
   without link-time optimisation a chatty boundary is thousands of calls that are not inlined.

Rule 3 is the one whose breach is invisible, so it is pinned in the source:
`legacy/map_engine/src/frame/tests/damage_discipline.rs` reads `frame/lifecycle.rs`,
`frame/encode.rs` and `frame/engine.rs` at compile time and fails if `render` stops refusing an
undamaged frame (`render_refuses_to_submit_an_undamaged_frame`), a lane mutation stops marking
damage (`every_lane_mutation_marks_the_frame_damaged`), the packet stops reading the persistent
list (`the_packet_reads_the_persistent_batch_list`) or the tables are rebuilt rather than refilled
(`the_packets_lookup_tables_are_refilled_not_rebuilt`). `RenderEngine` needs a GPU device, so no
native test can build one; the source pin is the proof that remains. The damage state machine
itself is tested in `render_primitives` (`crates/graphics/render_primitives/src/frame/tests/`).

### 2C.1 What may name a graphics type

The map engine names the graphics engine's modules in three kinds, and each kind has its own
rule:

- **Kind A: GPU resource creation.** Device buffers, shader modules, atlases, pipelines and the
  render loop belong to the graphics engine (`device`, `pipeline`, `shaders`, `r#loop`). The map
  engine receives built handles and does not name these modules, except at five pinned sites that
  exist because `RenderEngine`, which holds every GPU resource, is defined in the map engine
  ([rule 3b](#rule-3b-no-gpu-resource-module-in-the-map-engine)).
- **Kind B: byte layouts and packing.** The shared binary contract between the crates (instance
  layouts, vertex and geometry packing, text metrics and layout, the frame ids, the camera
  uniform and damage tracking) is the `render_primitives` crate, which carries no GPU handle and
  is imported directly by the upload belts that sit next to their data, in `world/`, `overlay/`,
  `spatial/`, `frame/` and `diagnostics/`. No rule restricts Kind B: routing it through `frame/`
  would put vegetation and symbology knowledge into `frame/` and make it a god module.
- **Kind C: the packet vocabulary.** `FramePacket`, `DrawBatch`, `DrawPayload`, `InstanceBuffer`,
  `TextRun` and the rest of the graphics engine's `frame` module, every item of which names a
  `wgpu` type, reach the map engine through `crate::frame` only, re-exported by name (never a glob) in
  `legacy/map_engine/src/frame/mod.rs`, so the crate's whole graphics interface is one list
  a reviewer reads on one screen, and widening it is a diff to that list
  ([rule 3a](#rule-3a-only-the-packet-boundary-names-the-frame-vocabulary)). `frame/`'s own
  submodules import `crate::frame::DrawBatch` like every other module.

### 2D Static world and authored document

`world/` is immutable, streamed from `assets/terrains/`, cacheable and never persisted. The
mission document (`mission_crdt`, `mission_document`, `mission_operations`, hosted by `editing/`)
is mutable, undoable, CRDT-synced and persisted. They share nothing: a `world/` type never gains a
dirty flag, and a document type never gains a chunk id. The two meet only in `editing/`, which
hosts and drives the document and asks the `spatial_indexes` and line of sight crates where a
click or a sight line lands.

[Rule 7](#rule-7-the-static-world-names-no-home-of-the-authored-document) gates the world's side
of the import wall, which is what can be checked exactly: a `world/` type cannot gain undo,
persistence or CRDT state without naming `yrs`, a mission document crate or `crate::editing`. The
document's side is a crate boundary ([2A](#2a-the-mission-crates-stand-outside-the-map-engine)): a
mission crate cannot name a chunk id, tile coordinate, level of detail or residency handle because
it cannot depend on the crate that declares it. The gate does not see a local `dirty: bool` on a
world struct that imports nothing; it catches the flag when anything tries to persist it.

## 3. The graphics engine: a pure renderer

`graphics_engine` binds and draws what it is told. It defines the GPU frame vocabulary, and
its five modules (`device`, `draw`, `frame`, `loop`, `pipeline`) hold GPU handles and uploads
only; the GPU-free geometry, byte layouts, glyph text and WGSL source it builds on are the
`render_primitives` crate. The map engine decides what to draw, in which
order and with which pipeline, and keys every batch on an opaque `LaneId`; the lane roles and
their paint order live in `crates/map_overlay/map_draw_lanes/src/lane_roles.rs`. A map noun in a
declared name means a domain decision crossed the wall: move the decision to the map engine and
leave the packing in the renderer. The [graphics engine overview](/documentation/legacy/graphics_engine/graphics_engine_overview.md)
describes the crate.

## 4. The frontend: a thin browser app

The frontend owns presentation, input and the browser shell. It reaches the renderer through
`map_engine` only, because the map engine owns the frame vocabulary (rule 3a) and the GPU
resources (rule 3b): a frontend that imported the renderer would make both walls optional. It
supplies what only a browser has (the canvas, preference readers, clocks, storage) to the map
engine as values and closures, and keeps the state [section 1](#where-state-lives) gives it.

## 5. Gate rules

`cargo xtask verify engine-layers` checks the rules below. It runs as the `verify-engine-layers`
step of `cargo xtask ci ci-local` (`tools/xtask/src/commands/ci/task_definitions.rs:48`) and as
a step of the `language-gates` job in `.github/workflows/ci.yml`, beside the language gates: it
reads the working tree, needs no database, LFS object or wasm target, and takes seconds.
`cargo xtask ci verify-engine-layers` is an alias of the same verb. The code is in
`tools/foundation/repository_laws/src/engine_layers/`: the rule catalogue and its
reasons in `mod.rs`, the matchers, pins and scanned roots in `rules.rs`, the head lines and
messages in `report_text.rs`, the walks in `crate_walks.rs` and the pin arithmetic in
`scanning.rs`, as its [README](/tools/foundation/repository_laws/src/engine_layers/README.md)
lists. `tools/xtask/src/verifications/architecture/engine_layer_boundaries.rs` prints that
library's report for `cargo xtask verify engine-layers`, and the `engineering_laws` test binary of
`api` reads the same library; the
[architecture gates README](/tools/xtask/src/verifications/architecture/README.md) summarises
all three architecture gates.

### How the gate judges

- **Inputs.** It walks the `.rs` files under `legacy/graphics_engine/src`, under the `src`
  folder of every workspace member whose category is `crates/graphics` (read from the root
  `Cargo.toml` members), under `legacy/map_engine/src` and under `apps/frontend/src`, and reads
  the manifests of the graphics engine, of each graphics member and of the frontend. Untracked
  files count. A folder below the repository root named `target` or starting with `target-` is
  build output and is pruned.
- **Self-probes.** Every matcher is a compiled constant, except rule 6's arm over the wasm-only
  graphics crates, which is built from their crate names. Before any matcher judges source it
  runs over subjects whose answer is known, positive and negative. A probe that answers wrongly fails the
  gate: a check whose matcher is broken is not a pass.
- **Matching.** Each matcher is a line matcher over source text. Rules 1, 3a, 3b, 5 and 7 see
  prose as well as code, on purpose; rules 2 and 6 match syntax, so prose describing the boundary
  passes.
- **Pins.** Rules 3a and 3b allow enumerated sites: a file, its exact count and the reason. A
  pin is a ratchet. An unpinned file that matches fails; a pinned file whose count rises or falls
  fails; a pinned file that no longer exists fails.
- **Exit codes.** 0 when every rule holds; 1 when a rule is broken, a self-probe fails, a
  scanned root holds no `.rs` file or the graphics category has no member (a vacuous pass is
  refused); 2 when a root, a manifest, the workspace member list or a file could not be read, because "I never looked" is a different action from "I looked and it is
  dirty".

| Rule | Subject | Must hold |
|---|---|---|
| 1 | `legacy/graphics_engine` and each `crates/graphics` member | no `map_engine::` path or `extern crate map_engine` in source, no `map_engine` edge in `Cargo.toml` |
| 2 | `legacy/graphics_engine` and each `crates/graphics` member | no declared name (after `struct`, `enum`, `trait`, `type`, `fn`, `const`, `static`, `mod`) containing terrain, symbology, mission, orbat or arma, case-insensitive |
| 3a | `legacy/map_engine/src` | `graphics_engine::frame` appears only in `frame/mod.rs`, exactly 5 times |
| 3b | `legacy/map_engine/src` | `graphics_engine::` followed by `device`, `pipeline`, `shaders` or `r#loop` appears only at the pinned sites: 3 in `frame/mod.rs`, 2 in `frame/pump.rs` |
| 5 | `editing` | no `web_sys`, `leptos` or `wasm_bindgen`, prose included |
| 6 | `apps/frontend` | no `graphics_engine::` path or `extern crate`, no `graphics_engine` edge in `Cargo.toml`; the same for each `crates/graphics` member declaring `targets = "wasm32"` |
| 7 | `world` | no `yrs::` path, no `mission_crdt::`, `mission_document::` or `mission_operations::` path, no `crate::editing` and no `super::` chain ending on `editing` |

The paths of rules 5 and 7 are under `legacy/map_engine/src/`. Number 4 is unassigned: the
mission crates' isolation is the crate-tier law's
([2A](#2a-the-mission-crates-stand-outside-the-map-engine)).

### Rule 1: the graphics layer never imports the map engine

- Subject: the `.rs` files under `legacy/graphics_engine/src` and its `Cargo.toml`, and the
  `.rs` files under the `src` folder and the `Cargo.toml` of every `crates/graphics` member.
- Forbids: a `map_engine::` path or `extern crate map_engine` in source
  (`MAP_ENGINE_IMPORT_RE`), and `map_engine` on a manifest line that is not a `#` comment. The
  manifest arm closes the rename hole: a dependency renamed with `package = "map_engine"` would
  make every `use` spell a name the source arm never sees.
- Why: [dependency direction](#dependency-direction--non-negotiable). A reverse edge turns the
  arrow into a cycle.
- Agreement with the crate-tier law: the category matrix lets a graphics crate depend on
  foundation and graphics crates only, and the strangler rule forbids any edge into `legacy/`, so
  a `crates/graphics` member's edge to the map engine or to a map crate is already a crate-tier
  finding. Rule 1 forbids a subset of the same edges and adds the source arm; the two never
  disagree. The legacy graphics engine is outside the crate-tier law, and rule 1 with the
  `GRAPHICS_ENGINE_RULE` row of `tools/foundation/repository_laws/src/crate_dependencies.rs`
  are its wall.
- Exemptions and pins: none.

### Rule 2: no map noun in a graphics layer declaration

- Subject: the `.rs` files under `legacy/graphics_engine/src` and under the `src` folder of
  every `crates/graphics` member. The crate-tier law's graphics firewall reuses the same matcher
  over the category, so the regex stays when the legacy engine is deleted.
- Forbids: a declaration keyword (`struct`, `enum`, `trait`, `type`, `fn`, `const`, `static`,
  `mod`), whitespace, then a name containing `terrain`, `symbology`, `mission`, `orbat` or
  `arma`, case-insensitive (`DECL_RE`). `struct TerrainBlob` and `fn submit_mission` fail;
  "a submission arrives here" in a comment and `include_str!("terrain.wgsl")` pass.
- Why: the nouns are ordinary English and graphics words too, and a gate that fires on prose gets
  suppressed; a noun inside a declared name is domain logic that came back across the wall
  ([section 3](#3-the-graphics-engine-a-pure-renderer)).
- Exemptions and pins: none. The matcher does not see enum variants or struct fields, which carry
  no keyword; the lane roles, the one such population, are opaque `LaneId`s on the graphics side.

### Rule 3a: only the packet boundary names the frame vocabulary

- Subject: the `.rs` files under `legacy/map_engine/src`.
- Forbids: `graphics_engine::frame` followed by a word boundary (`FRAME_VOCAB_RE`), in
  code or prose, outside the pin. `crate::frame::DrawBatch` and a module named `frames` do not
  match.
- Why: Kind C of [2C.1](#2c1-what-may-name-a-graphics-type). A directory rule ("anything under
  `frame/`") would pass thirteen files each importing what they liked; a per-file pin keeps the
  interface one list.
- Pin (`RULE3A_PIN`): `legacy/map_engine/src/frame/mod.rs`, 5 lines, the enumerated packet
  vocabulary. The file's own prose never spells the path, so the count is exactly the size of the
  interface list.

### Rule 3b: no GPU-resource module in the map engine

- Subject: the `.rs` files under `legacy/map_engine/src`.
- Forbids: `graphics_engine::` followed by `device`, `pipeline`, `shaders` or `r#loop`
  and a word boundary (`GPU_MODULE_RE`), outside the pins.
- Why: Kind A of [2C.1](#2c1-what-may-name-a-graphics-type). These modules create and own GPU
  resources; the fix for a new site is to move the construction into the graphics engine, not to
  add a pin row.
- Pins (`RULE3B_PIN`), all caused by `RenderEngine` living in the map engine:
  - `legacy/map_engine/src/frame/mod.rs`, 3: the `device::buffers` and `pipeline` aliases,
    one seam each for 3 and 18 call sites, and the doc line on the `r#loop` re-export the
    frontend reaches the pump through;
  - `legacy/map_engine/src/frame/pump.rs`, 2: `impl FrameTarget for RenderEngine`, which
    must live in the crate that defines the type (E0116), and `#[wasm_bindgen]` refuses trait
    impls.
  Moving `RenderEngine` into the graphics engine is the only change that empties this list.

### Rule 5: no browser crate in the editing layer

- Subject: the `.rs` files under `legacy/map_engine/src/editing`.
- Forbids: the bare words `web_sys`, `leptos` and `wasm_bindgen` (`DOM_RE`), in code or prose;
  `web_sysfs` or `leptosaur` would not match.
- Why: [2B](#2b-the-headless-editing-layer). Inside this tree a comment telling the next reader to
  reach for `leptos` is the breach the rule exists to stop.
- Exemptions and pins: none; a hard zero.

### Rule 6: the frontend never imports the GPU layer

- Subject: the `.rs` files under `apps/frontend/src` and `apps/frontend/Cargo.toml`.
- Forbids: a `graphics_engine::` path or `extern crate graphics_engine`
  (`GRAPHICS_IMPORT_RE`), and `graphics_engine` on a manifest line that is not a `#`
  comment; and the same two shapes for every `crates/graphics` member declaring
  `targets = "wasm32"`, spelled with that member's crate and package names
  (`wasm_only_graphics_import_re`). Prose naming a crate while describing the boundary passes.
- Why: [section 4](#4-the-frontend-a-thin-browser-app). The rule keeps the frontend away from the
  GPU layer, not from shared CPU code: a graphics member built for every target
  (`targets = "any"`, such as `render_primitives`) holds byte layouts, geometry and glyph
  packing the frontend may link.
- Exemptions and pins: none.

### Rule 7: the static world names no home of the authored document

- Subject: the `.rs` files under `legacy/map_engine/src/world`.
- Forbids (`RULE7_WORLD_RE`): a `yrs::` path; a `mission_crdt::`, `mission_document::` or
  `mission_operations::` path; `crate::editing`; and a `super::` chain of any length ending on
  `editing`. `yrs::` rather than the bare word, so "3 yrs" in prose passes; `mission_model` and
  the other mission crates are not the document and pass.
- Why: [2D](#2d-static-world-and-authored-document). The `world` tier alone links no mission
  crate, but the `--all-features` build CI runs does, and nothing in the compiler stops `world/`
  naming the document there. The `super::` arm exists because a `crate::`-only matcher would leave
  a one-line bypass, and a depth count would be unsound where `#[path]` separates file depth from
  module depth.
- Exemptions and pins: none. The rule counts its files, and an empty `world/` fails.

### Changing a rule or a pin

- A pin row is a reviewed edit to
  `tools/foundation/repository_laws/src/engine_layers/rules.rs`, carrying its file,
  exact count and reason; the unit tests in
  `tools/foundation/repository_laws/src/engine_layers/tests/` hold each rule's shape (`naming_the_frame_vocabulary_outside_the_boundary_fails`,
  `a_new_gpu_module_import_in_the_map_engine_fails`,
  `the_world_naming_the_document_breaches_the_wall`,
  `inputs_that_were_never_read_do_not_pass`).
- A new rule gets a number in this section, a matcher with its self-probes, a head line and a tail
  message citing this document, and a row in the table above.
- Moving a scanned folder moves the gate's path constants in the same commit; otherwise the gate
  refuses with exit 2 or 1, never passes.

## 6. Known gaps and open work

- **Rule 2's noun list is short.** Declared names such as `BuildingInstance`,
  `create_building_pipeline`, `create_forest_density_pipeline` and `create_map_shader` name map
  concepts the five nouns do not catch (`crates/graphics/render_primitives/src/draw/instances.rs`,
  `pipeline/`).
- **The wave gate does not run it.** `VERIFY_STEPS` in
  `tools/xtask/src/commands/platform/wave_execution/gate.rs:59` has no engine-layers row; only
  `ci-local` and CI run the gate.

Tickets:

- [T-1070 — Remove map nouns from graphics engine names; widen rule 2](/.ai/tickets/T-1070.toml)
  (idea, no plan): renames the graphics engine's map-named items to geometry terms and adds
  building, forest, map, hillshade, slot, town and road to rule 2's nouns.
- [T-1130 — Tidy xtask verifications: redundant check, scattered paths, wave gate](/.ai/tickets/T-1130.toml)
  (idea, no plan): moves the gate's hard-coded paths into the repository layout module and adds
  engine-layers to the wave gate.

## Related documentation

- [Map engine documentation](/documentation/legacy/map_engine/README.md) — the crate's layers,
  streaming, editing layer and draft persistence.
- [Graphics engine documentation](/documentation/legacy/graphics_engine/README.md) — the pure
  renderer.
- [Architecture gates](/tools/xtask/src/verifications/architecture/README.md) — the gate's
  inputs, exit codes and unit tests.
- [Engine split program](/documentation/archive/engine_split/engine_split_program.md) — the
  archived program these rules come from.
- [Coding standards](/documentation/standards/coding_standards/README.md) — the repository's
  other code rules.
