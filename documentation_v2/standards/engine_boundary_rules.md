**Status:** live

# Engine boundary rules

The layer rules between the website's two engine crates and the app that uses them: which crate
may name which, where state lives, how the map engine hands frames to the renderer, and the walls
inside the map engine. `cargo xtask verify engine-layers` enforces the rules section 5 numbers,
and its failure messages, the CI step and the `ci-local` task cite this document by section. The
code is the final word: the gate's matchers and pins in
`tools_v2/xtask/src/verifications/architecture/` define each rule exactly, and this document
states them and the reasons behind them.

## 1. Layers and dependency direction

The web platform draws its map with three crates, each with one job:

| Crate | Code | Job |
|---|---|---|
| `website-graphics-engine` | [`apps/website/graphics-engine/`](/apps/website/graphics-engine/README.md) | the renderer: device buffers, pipelines, the WGSL shader, draw batching, text packing, sprite culling and the animation-frame pump; it knows no map concept |
| `website-map-engine` | [`apps/website/map-engine/`](/apps/website/map-engine/README.md) | the [mission](/documentation_v2/glossary.md#mission) domain, the static world, streaming, spatial queries, the overlay, the cameras, the headless editing layer and `RenderEngine`, which builds each frame |
| `website-frontend` | [`apps/website/frontend/`](/apps/website/frontend/README.md) | the single-page app, including the [Mission Creator](/documentation_v2/glossary.md#mission-creator): pages, UI, input and the browser shell |

The [API](/documentation_v2/glossary.md#api) (`website-api`) links the map engine too, for the
mission compiler and validator alone.

### Dependency direction — non-negotiable

```text
website-api      ──► website-map-engine {scenario}                  native, no GPU crate
website-frontend ──► website-map-engine {world, io, store, editing}  every target
                                        {render, streaming}          wasm32
                     website-map-engine ──► website-graphics-engine  from the world tier up
```

- The arrow runs from the map engine to the graphics engine and never back. The graphics engine
  never learns a map noun; the map engine speaks the renderer's frame vocabulary, not the reverse
  ([rule 1](#rule-1-the-graphics-engine-never-imports-the-map-engine),
  [rule 2](#rule-2-no-map-noun-in-a-graphics-engine-declaration)).
- The frontend never depends on the graphics engine. It hands the map engine a canvas, and
  reaches the render loop through the map engine's re-export of `RafPump` and `FrameTarget`
  (`apps/website/map-engine/src/frame/mod.rs`, `frame/pump.rs`)
  ([rule 6](#rule-6-the-frontend-never-imports-the-graphics-engine)).
- Nothing in the compiler enforces either direction: a dependency edge pointing back would still
  build. The gate is the only wall.

The graphics engine's own layout and public surface are in its
[README](/apps/website/graphics-engine/README.md); the map engine's modules and feature tiers are
in its [README](/apps/website/map-engine/README.md) and [source README](/apps/website/map-engine/src/README.md).

### Where state lives

> The map engine owns state that survives a reload. The frontend owns state that dies with the
> tab.

| Survives a reload: `website-map-engine` | Dies with the tab: `website-frontend` |
|---|---|
| the mission document, its entities and its undo stack (`data/store/`, `editing/history/`) | hover and drag in progress |
| the selection (`editing/host.rs`) | the pointer gesture state machine (`editor/input/`) |
| the tool command definitions (`editing/commands/`, `editing/tools/`) | the keybind map |
| the line-of-sight, ruler and viewshed algorithms (`spatial/`, `editing/tools/`) | panels open or closed, dock sizes |
| the decisions behind serialising and hydrating a draft (`editing/persist/`) | the tab lock, the save-status signal and the title (`editor/shell/`) |

A file whose home is unclear goes where this rule sends it. The frontend paths are under
`apps/website/frontend/src/v2/apps/editor/`; the map engine paths under
`apps/website/map-engine/src/`. Where the bytes of a draft are stored (IndexedDB) and how they
travel is the frontend's; what they mean is the map engine's.

### Feature tiers

A consumer takes only the tier it needs; the feature table and its consumers are in the map
engine README's [Configuration](/apps/website/map-engine/README.md#configuration). Three facts of
the tiers are boundary rules:

- `scenario`, the default, is the tier the API links, and it pulls no graphics crate, PNG
  decoder, `rkyv` or `flate2`. [Rule 4](#rule-4-the-mission-compiler-imports-nothing-outside-itself)
  keeps it that way.
- `world` turns the graphics engine on, not `render` alone (`apps/website/map-engine/Cargo.toml:34`):
  the upload belts of `world/`, `overlay/` and `spatial/` import the renderer's byte layouts and
  geometry helpers directly (Kind B in [2C.1](#2c1-what-may-name-a-graphics-type)), and the
  frontend's debug building viewer calls `website_map_engine::world::mesh::triangulate`, the
  map engine's re-export of `website_graphics_engine::draw::triangulate`
  (`apps/website/map-engine/src/world/mesh.rs:17`).
- `editing` takes `store`, `world` and `streaming`, because the line-of-sight tool tests cells
  against the streamed world's occluder under `spatial/los/world/`
  (`apps/website/map-engine/Cargo.toml:43`).

## 2. Walls inside the map engine

The map engine is one crate of eleven top-level modules (`apps/website/map-engine/src/lib.rs`).
Four walls inside it replace the crate boundaries a split into more crates would have drawn.

### 2A The API's thin tier

`data/scenario/` is the mission compiler and validator, the only part of the crate the API
compiles. It names nothing outside itself: no other top-level module, not `data::store`, not the
graphics engine. One import of `crate::streaming` inside it would drag the streaming tier, and
with it `rkyv`, `flate2`, `png` and the graphics engine, into an HTTP server. A value it needs
from elsewhere is passed in as an argument. [Rule 4](#rule-4-the-mission-compiler-imports-nothing-outside-itself)
enforces it.

### 2B The headless editing layer

`editing/` holds the Mission Creator's decisions: the editing host, the hosted commands, the undo
drive, the draft decisions and the tool state machines. Every one of them is answerable by
`cargo test` with no browser. What only a host can supply (a clock, a frame pump, a prompt, a
storage read) crosses as an injected closure or function pointer, so the tree names no browser
crate. [Rule 5](#rule-5-no-browser-crate-in-the-editing-layer) enforces it; the layer itself is
described in the [editing layer doc](/documentation_v2/website/map-engine/editing_layer.md).

The rest of the crate is a wasm crate that reaches the browser on purpose (the streaming host,
the readback diagnostics, the doll renderer, the render engine), so the browser ban covers
`editing/` alone.

### 2C The packet boundary

`apps/website/map-engine/src/frame/` builds the graphics engine's `FramePacket` from the world,
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
   (`apps/website/graphics-engine/src/frame/batch.rs`, `frame/buffers.rs`), never owned geometry.
   Owned geometry would copy it every frame.
3. **Damage tracking survives.** `render` returns before acquiring a surface texture when
   `RenderDamage` says the frame is clean and not continuous, and every lane mutation marks the
   frame damaged. If packet building became "walk the world, rebuild every batch", the
   damage-driven renderer would turn immediate-mode, and every test would still pass.
4. **One call per frame across the crate boundary.** `encode_main_pass` hands the whole packet to
   `website_graphics_engine::draw::encode::encode` once a frame, never per batch or per instance:
   without link-time optimisation a chatty boundary is thousands of calls that are not inlined.

Rule 3 is the one whose breach is invisible, so it is pinned in the source:
`apps/website/map-engine/src/frame/tests/damage_discipline.rs` reads `frame/lifecycle.rs`,
`frame/encode.rs` and `frame/engine.rs` at compile time and fails if `render` stops refusing an
undamaged frame (`render_refuses_to_submit_an_undamaged_frame`), a lane mutation stops marking
damage (`every_lane_mutation_marks_the_frame_damaged`), the packet stops reading the persistent
list (`the_packet_reads_the_persistent_batch_list`) or the tables are rebuilt rather than refilled
(`the_packets_lookup_tables_are_refilled_not_rebuilt`). `RenderEngine` needs a GPU device, so no
native test can build one; the source pin is the proof that remains. The damage state machine
itself is tested in the graphics engine (`apps/website/graphics-engine/src/frame/tests/`).

### 2C.1 What may name a graphics type

The map engine names the graphics engine's modules in three kinds, and each kind has its own
rule:

- **Kind A: GPU resource creation.** Device buffers, shader modules, atlases, pipelines and the
  render loop belong to the graphics engine (`device`, `pipeline`, `shaders`, `r#loop`). The map
  engine receives built handles and does not name these modules, except at five pinned sites that
  exist because `RenderEngine`, which holds every GPU resource, is defined in the map engine
  ([rule 3b](#rule-3b-no-gpu-resource-module-in-the-map-engine)).
- **Kind B: byte layouts and packing.** The shared binary contract between the crates (instance
  layouts, vertex and geometry packing, text metrics and layout) is imported directly by the
  upload belts that sit next to their data, in `world/`, `overlay/`, `spatial/` and
  `diagnostics/`. The graphics engine publishes it as one list, `website_graphics_engine::layout`,
  beside `draw` and `text`. No rule restricts Kind B: routing it through `frame/` would put
  vegetation and symbology knowledge into `frame/` and make it a god module.
- **Kind C: the packet vocabulary.** `FramePacket`, `DrawBatch`, `DrawPayload`, `InstanceBuffer`,
  `LaneId`, `CameraUniform` and the rest of the graphics engine's `frame` module reach the map
  engine through `crate::frame` only, re-exported by name (never a glob) in
  `apps/website/map-engine/src/frame/mod.rs`, so the crate's whole graphics interface is one list
  a reviewer reads on one screen, and widening it is a diff to that list
  ([rule 3a](#rule-3a-only-the-packet-boundary-names-the-frame-vocabulary)). `frame/`'s own
  submodules import `crate::frame::DrawBatch` like every other module.

### 2D Static world and authored document

`world/` is immutable, streamed from `assets_v2/terrains/`, cacheable and never persisted. `data/`
is mutable, undoable, CRDT-synced and persisted. They share the spatial index and nothing else: a
`world/` type never gains a dirty flag, and a `data/` type never gains a chunk id. Inside the
crate the two meet only in `editing/`, which drives the document and asks `spatial/` where a
click or a sight line lands.

[Rule 7](#rule-7-the-static-world-and-the-authored-document-share-nothing) gates the import wall,
which is what can be checked exactly: a `world/` type cannot gain undo, persistence or CRDT state
without naming `crate::data` or `yrs`, and a `data/` type cannot gain a chunk id, tile coordinate,
level of detail or residency handle without naming the module that declares it. The gate does not
see a local `dirty: bool` on a world struct that imports nothing; it catches the flag when
anything tries to persist it.

## 3. The graphics engine: a pure renderer

`website-graphics-engine` binds and draws what it is told. It defines the frame vocabulary, and
its eight modules (`device`, `draw`, `frame`, `layout`, `loop`, `pipeline`, `shaders`, `text`)
hold geometry, GPU handles and byte layouts only. The map engine decides what to draw, in which
order and with which pipeline, and keys every batch on an opaque `LaneId`; the lane roles and
their paint order live in `apps/website/map-engine/src/overlay/lanes.rs`. A map noun in a
declared name means a domain decision crossed the wall: move the decision to the map engine and
leave the packing in the renderer. The [graphics engine overview](/documentation_v2/website/graphics-engine/graphics_engine_overview.md)
describes the crate.

## 4. The frontend: a thin browser app

The frontend owns presentation, input and the browser shell. It reaches the renderer through
`website-map-engine` only, because the map engine owns the frame vocabulary (rule 3a) and the GPU
resources (rule 3b): a frontend that imported the renderer would make both walls optional. It
supplies what only a browser has (the canvas, preference readers, clocks, storage) to the map
engine as values and closures, and keeps the state [section 1](#where-state-lives) gives it.

## 5. Gate rules

`cargo xtask verify engine-layers` checks the rules below. It runs as the `verify-engine-layers`
step of `cargo xtask ci ci-local` (`tools_v2/xtask/src/commands/ci/task_definitions.rs:46`) and as
a step of the `language-gates` job in `.github/workflows/ci.yml`, beside the language gates: it
reads the working tree, needs no database, LFS object or wasm target, and takes seconds.
`cargo xtask ci verify-engine-layers` is an alias of the same verb. The code is in
`tools_v2/xtask/src/verifications/architecture/`: the rules and their reasons in
`engine_layer_boundaries.rs`, the matchers, pins and messages in `engine_layer_rules.rs`, the
walks and pin arithmetic in `engine_layer_scan.rs`; the
[architecture gates README](/tools_v2/xtask/src/verifications/architecture/README.md) summarises
all three architecture gates.

### How the gate judges

- **Inputs.** It walks the `.rs` files under `apps/website/graphics-engine/src`,
  `apps/website/map-engine/src` and `apps/website/frontend/src`, and reads
  `apps/website/graphics-engine/Cargo.toml` and `apps/website/frontend/Cargo.toml`. Untracked
  files count. A folder below the repository root named `target` or starting with `target-` is
  build output and is pruned.
- **Self-probes.** Every matcher is a compiled constant, and before it judges source it runs over
  subjects whose answer is known, positive and negative. A probe that answers wrongly fails the
  gate: a check whose matcher is broken is not a pass.
- **Matching.** Each matcher is a line matcher over source text. Rules 1, 3a, 3b, 4, 5 and 7 see
  prose as well as code, on purpose; rules 2 and 6 match syntax, so prose describing the boundary
  passes.
- **Pins.** Rules 3a, 3b and 4 allow enumerated sites: a file, its exact count and the reason. A
  pin is a ratchet. An unpinned file that matches fails; a pinned file whose count rises or falls
  fails; a pinned file that no longer exists fails.
- **Exit codes.** 0 when every rule holds; 1 when a rule is broken, a self-probe fails, or a
  scanned root holds no `.rs` file (a vacuous pass is refused); 2 when a root, a manifest or a
  file could not be read, because "I never looked" is a different action from "I looked and it is
  dirty".

| Rule | Subject | Must hold |
|---|---|---|
| 1 | `apps/website/graphics-engine` | no `website_map_engine` in source, no `website-map-engine` edge in `Cargo.toml` |
| 2 | `apps/website/graphics-engine` | no declared name (after `struct`, `enum`, `trait`, `type`, `fn`, `const`, `static`, `mod`) containing terrain, symbology, mission, orbat or arma, case-insensitive |
| 3a | `apps/website/map-engine/src` | `website_graphics_engine::frame` appears only in `frame/mod.rs`, exactly 8 times |
| 3b | `apps/website/map-engine/src` | `website_graphics_engine::` followed by `device`, `pipeline`, `shaders`, `r#loop` or `text::gpu` appears only at the pinned sites: 3 in `frame/mod.rs`, 2 in `frame/pump.rs` |
| 4 | `data/scenario` | names none of `crate::` `camera`, `diagnostics`, `doll`, `frame`, `io`, `overlay`, `spatial`, `streaming`, `world`, `data::store`, nor `website_graphics_engine`, nor a `super::` chain ending on one of them (`diagnostics` excepted), outside two pinned `cfg(feature = "store")` test files |
| 5 | `editing` | no `web_sys`, `leptos` or `wasm_bindgen`, prose included |
| 6 | `apps/website/frontend` | no `website_graphics_engine::` path or `extern crate`, no `website-graphics-engine` edge in `Cargo.toml` |
| 7 | `data` and `world` | `data/` names none of the nine sibling modules of rule 4 nor the graphics engine; `world/` names neither `crate::data` nor `yrs::` |

The paths of rules 4, 5 and 7 are under `apps/website/map-engine/src/`.

### Rule 1: the graphics engine never imports the map engine

- Subject: the `.rs` files under `apps/website/graphics-engine/src` and its `Cargo.toml`.
- Forbids: the text `website_map_engine` in source, and `website-map-engine` on a manifest line
  that is not a `#` comment. The manifest arm closes the rename hole: a dependency renamed with
  `package = "website-map-engine"` would make every `use` spell a name the source arm never sees.
- Why: [dependency direction](#dependency-direction--non-negotiable). A reverse edge turns the
  arrow into a cycle.
- Exemptions and pins: none.

### Rule 2: no map noun in a graphics engine declaration

- Subject: the `.rs` files under `apps/website/graphics-engine/src`.
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

- Subject: the `.rs` files under `apps/website/map-engine/src`.
- Forbids: `website_graphics_engine::frame` followed by a word boundary (`FRAME_VOCAB_RE`), in
  code or prose, outside the pin. `crate::frame::DrawBatch` and a module named `frames` do not
  match.
- Why: Kind C of [2C.1](#2c1-what-may-name-a-graphics-type). A directory rule ("anything under
  `frame/`") would pass thirteen files each importing what they liked; a per-file pin keeps the
  interface one list.
- Pin (`RULE3A_PIN`): `apps/website/map-engine/src/frame/mod.rs`, 8 lines, the enumerated packet
  vocabulary. The file's own prose never spells the path, so the count is exactly the size of the
  interface list.

### Rule 3b: no GPU-resource module in the map engine

- Subject: the `.rs` files under `apps/website/map-engine/src`.
- Forbids: `website_graphics_engine::` followed by `device`, `pipeline`, `shaders`, `r#loop` or
  `text::gpu` and a word boundary (`GPU_MODULE_RE`), outside the pins.
- Why: Kind A of [2C.1](#2c1-what-may-name-a-graphics-type). These modules create and own GPU
  resources; the fix for a new site is to move the construction into the graphics engine, not to
  add a pin row.
- Pins (`RULE3B_PIN`), all caused by `RenderEngine` living in the map engine:
  - `apps/website/map-engine/src/frame/mod.rs`, 3: the `device::buffers` and `pipeline` aliases,
    one seam each for 3 and 18 call sites, and the doc line on the `r#loop` re-export the
    frontend reaches the pump through;
  - `apps/website/map-engine/src/frame/pump.rs`, 2: `impl FrameTarget for RenderEngine`, which
    must live in the crate that defines the type (E0116), and `#[wasm_bindgen]` refuses trait
    impls.
  Moving `RenderEngine` into the graphics engine is the only change that empties this list.

### Rule 4: the mission compiler imports nothing outside itself

- Subject: the `.rs` files under `apps/website/map-engine/src/data/scenario`.
- Forbids (`RULE4_RE`): `crate::` followed by `camera`, `diagnostics`, `doll`, `frame`, `io`,
  `overlay`, `spatial`, `streaming`, `world` or `data::store`; `website_graphics_engine`; and a
  `super::` chain of any length ending on `store`, `camera`, `doll`, `frame`, `io`, `overlay`,
  `spatial`, `streaming` or `world`. `diagnostics` is left out of the `super::` arm because
  `data/scenario/compiler/flatten/diagnostics.rs` is a module inside the tree.
- Why: [2A](#2a-the-apis-thin-tier). The `super::` arm exists because a `crate::`-only matcher
  would leave a one-line bypass, and a depth count would be unsound where `#[path]` separates file
  depth from module depth.
- Pins (`RULE4_PIN`), both `#[cfg(feature = "store")]` test code the API never compiles:
  `data/scenario/compiler/flatten/tests/mod.rs` (1, `vehicles_from_writer_json_roundtrip`) and
  `data/scenario/compiler/payload/tests/cases_1.rs` (1,
  `briefing_prose_round_trips_through_the_document_core`). An ungated import would satisfy the
  count and still be wrong, which is why each pin carries its reason.

### Rule 5: no browser crate in the editing layer

- Subject: the `.rs` files under `apps/website/map-engine/src/editing`.
- Forbids: the bare words `web_sys`, `leptos` and `wasm_bindgen` (`DOM_RE`), in code or prose;
  `web_sysfs` or `leptosaur` would not match.
- Why: [2B](#2b-the-headless-editing-layer). Inside this tree a comment telling the next reader to
  reach for `leptos` is the breach the rule exists to stop.
- Exemptions and pins: none; a hard zero.

### Rule 6: the frontend never imports the graphics engine

- Subject: the `.rs` files under `apps/website/frontend/src` and `apps/website/frontend/Cargo.toml`.
- Forbids: a `website_graphics_engine::` path or `extern crate website_graphics_engine`
  (`GRAPHICS_IMPORT_RE`), and `website-graphics-engine` on a manifest line that is not a `#`
  comment. Prose naming the crate while describing the boundary passes.
- Why: [section 4](#4-the-frontend-a-thin-browser-app).
- Exemptions and pins: none.

### Rule 7: the static world and the authored document share nothing

- Subject: the `.rs` files under `apps/website/map-engine/src/data` and
  `apps/website/map-engine/src/world`, judged as one rule with one findings list.
- Forbids: under `data/` (`RULE7_DATA_RE`), `crate::` followed by `camera`, `diagnostics`, `doll`,
  `frame`, `io`, `overlay`, `spatial`, `streaming` or `world`, `website_graphics_engine`, or a
  `super::` chain ending on one of them but `diagnostics`; under `world/` (`RULE7_WORLD_RE`),
  `crate::data`, `yrs::` or a `super::` chain ending on `data`. `yrs::` rather than the bare word,
  so "3 yrs" in prose passes.
- Why: [2D](#2d-static-world-and-authored-document). A build of `--features store` alone compiles
  `data/` without the world modules, but the `--all-features` build CI runs does not, and nothing
  in the compiler stops `world/` naming `data/`.
- Exemptions and pins: none. Both sides count their files, and an empty `data/` or `world/` fails.

### Changing a rule or a pin

- A pin row is a reviewed edit to `engine_layer_rules.rs`, carrying its file, exact count and
  reason; the unit tests in `tools_v2/xtask/src/verifications/architecture/tests/` hold each
  rule's shape (`naming_the_frame_vocabulary_outside_the_boundary_fails`,
  `a_new_gpu_module_import_in_the_map_engine_fails`,
  `the_scenario_tree_reaching_outside_itself_fails`,
  `inputs_that_were_never_read_do_not_pass`).
- A new rule gets a number in this section, a matcher with its self-probes, a head line and a tail
  message citing this document, and a row in the table above.
- Moving a scanned folder moves the gate's path constants in the same commit; otherwise the gate
  refuses with exit 2 or 1, never passes.

## 6. Known gaps and open work

- **Rules 4 and 7 omit `editing`.** Their matchers list nine sibling modules, and the gate's
  comments say the crate has ten top-level modules (`engine_layer_rules.rs:108-111`,
  `engine_layer_boundaries.rs:72`); `lib.rs` declares eleven. `data/` or `data/scenario/` naming
  `crate::editing` passes today.
- **Rule 2's noun list is short.** Declared names such as `BuildingInstance`,
  `create_building_pipeline`, `create_forest_density_pipeline` and `create_map_shader` name map
  concepts the five nouns do not catch (`apps/website/graphics-engine/src/draw/instances.rs`,
  `pipeline/`).
- **A stale matcher arm.** Rule 3b matches `text::gpu`, a module the graphics engine does not have
  (`apps/website/graphics-engine/src/text/`); the arm can never fire.
- **Rule lists that omit rules 5 and 6.** The `verify-engine-layers` task help
  (`tools_v2/xtask/src/commands/ci/task_definitions.rs:293`), the `verify engine-layers` doc
  (`tools_v2/xtask/src/commands/verify/cli.rs:91`) and the CI step name and comment
  (`.github/workflows/ci.yml:260-261`) list rules 1, 2, 3a, 3b, 4 and 7; the gate runs all eight.
- **The wave gate does not run it.** `VERIFY_STEPS` in
  `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs:59` has no engine-layers row; only
  `ci-local` and CI run the gate.

Tickets:

- [T-1055 — Fix engine-layers gate omitting the map engine editing module](/.ai/tickets/T-1055.toml)
  (idea, no plan): adds `editing` to rules 4 and 7, fixes the module count, drops the `text::gpu`
  arm and names all eight rules in the help and CI texts.
- [T-1070 — Remove map nouns from graphics engine names; widen rule 2](/.ai/tickets/T-1070.toml)
  (idea, no plan): renames the graphics engine's map-named items to geometry terms and adds
  building, forest, map, hillshade, slot, town and road to rule 2's nouns.
- [T-1130 — Tidy xtask verifications: redundant check, scattered paths, wave gate](/.ai/tickets/T-1130.toml)
  (idea, no plan): moves the gate's hard-coded paths into the repository layout module and adds
  engine-layers to the wave gate.

## Related documentation

- [Map engine documentation](/documentation_v2/website/map-engine/README.md) — the crate's layers,
  streaming, editing layer and draft persistence.
- [Graphics engine documentation](/documentation_v2/website/graphics-engine/README.md) — the pure
  renderer.
- [Architecture gates](/tools_v2/xtask/src/verifications/architecture/README.md) — the gate's
  inputs, exit codes and unit tests.
- [Engine split program](/documentation_v2/archive/engine_split/engine_split_program.md) — the
  archived program these rules come from.
- [Coding standards](/documentation_v2/standards/coding_standards/README.md) — the repository's
  other code rules.
