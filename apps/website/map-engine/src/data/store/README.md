# Mission document store

The [mission](/documentation_v2/glossary.md#mission) as a live `yrs` CRDT document, the half of
the map engine's mission data that the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) edits: the document itself, the
CRDT pieces it is built from, and the headless operations that edit it. It sits behind the crate's
`store` feature and has no UI, browser or graphics dependency.

## Contents

```text
apps/website/map-engine/src/data/store/
├── crdt/          the native id arrays, the slot columns and the undo grouping clocks
├── mod.rs         the module tree; re-exports the surface of `rows/`, `crdt/` and `operations/`
├── operations/    the headless queries and edits the Mission Creator runs on the document
├── rows/          `MissionDocCore`: the root maps, every write, the JSON views, load, merge, undo
├── selection.rs   picked and marquee rows resolved to ids, slots first on equal distance
└── tests/         unit tests for the surface the store re-exports
```

## How it works

```text
Mission Creator (apps/website/frontend/src/v2/apps/editor/)
        │ crate::editing: the host, hosted commands, history, picking
        ▼
operations/   plan an edit in plain values; host decisions arrive as callbacks
        │ MissionDocCore mutators, one transaction each
        ▼
rows/         MissionDocCore: one yrs Doc, local and init origins, undo, JSON views, materialize
        │ id arrays, SlotSoa, grouping clock
        ▼
crdt/         native slotIds and entityIds arrays, the slot columns, the undo window and cap
```

The Mission Creator creates the document and shares its handle with `crate::editing`
(`editing::host::install`), whose hosted commands, undo history and tools edit it; the Mission
Creator also calls `operations` and the document's mutators directly. The compiler in
`crate::data::scenario` reads the document back through `small_maps_json` and `slots_json`; the
map lanes and the pickers read `materialize`'s `SlotSoa`. `crate::editing::picking` finds
candidate rows in the `SlotSoa` with the pick radius (`MissionDocCore::PICK_RADIUS_PX`) and grid
cell (`GRID_CELL_M`), and `selection.rs` maps them to ids: a
[slot](/documentation_v2/glossary.md#slot) and a vehicle at the same distance resolve to the slot,
and a marquee lists slots before vehicles.

`rows` and `selection` are private; they add their methods to `MissionDocCore` and leave the
module only through `mod.rs`. The host installs the one browser-side input the store needs, the
undo clock (`install_wasm_now`), so no browser binding enters this tree.

## Public surface

- `MissionDocCore` with `EntityTransformPatch`, `SquadMembership`, the connection types,
  `validate_connection_rows` and `formation_offsets`: for `crate::editing`, the Mission Creator
  and the crate's integration tests.
- `SlotSoa`, `NONE_IDX` and the `STANCE_*` codes: the slot projection `crate::editing` picks over
  and the Mission Creator draws.
- `install_wasm_now`, `GESTURE_WINDOW_MS`, `MAX_UNDO_GROUPS` and `ManualClock`: the clock hook the
  Mission Creator installs at wasm start, and the undo constants.
- `apply_faction_library` with its input, result and error types and the apply anchors, and
  `place_character_under_side` with `PlaceOrbatError`.
- `operations`, whose modules `crate::editing` and the Mission Creator call, and `crdt`.
- The selection methods on `MissionDocCore` (`pick_slot_or_vehicle`, `marquee_ids_with_vehicles`
  and their parts, `PICK_RADIUS_PX`, `GRID_CELL_M`), for `crate::editing::picking` and the
  selection tools.

## Boundaries

- Depends on: `crate::data::scenario` for the terrain bounds and the tactical graphic limits;
  `yrs`, `serde` and `serde_json`; nothing else of the crate.
- Used by:
  - `crate::editing`, the crate's integration tests in `apps/website/map-engine/tests/` and the
    store-gated tests of `apps/website/map-engine/src/data/scenario/compiler/`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/` and the DTOs of
    `apps/website/frontend/src/v2/core/api/dto/`;
  - the `engine-layers` and `editor-orbat-coherency` gates of `tools_v2/xtask/`, which scan and
    test this tree.
- Rules:
  - the tree names no crate module outside `crate::data` nor the graphics engine (rule 7 of
    `cargo xtask verify engine-layers`), and `crate::data::scenario`'s code never imports it
    (rule 4; two store-gated tests are pinned exceptions), so the
    [API](/documentation_v2/glossary.md#api), which links only `scenario`, carries no `yrs`;
  - the re-exported surface stays reachable through `data::store`
    (`connection_and_formation_api_is_crate_public_via_doc`,
    `entity_authoring_api_is_crate_public_via_doc` and
    `authoring_session_state_is_crate_public_via_doc` in `tests/reexports.rs`);
  - a slot beats a vehicle at equal distance
    (`square_slots_circular_vehicles_and_equal_distance_policy` in
    `apps/website/map-engine/src/editing/tests/picking_selection.rs`);
  - `cargo xtask verify editor-orbat-coherency` runs the store's
    [ORBAT](/documentation_v2/glossary.md#orbat) tests with `--features "scenario store"`.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the undo and redo, placement, layer and ORBAT features the document backs.
- [Architecture gates](/tools_v2/xtask/src/verifications/architecture/README.md) — the
  `engine-layers` and `editor-orbat-coherency` gates that scan this tree.
