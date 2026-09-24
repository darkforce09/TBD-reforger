# Mission document operations

The headless queries and edits the [Mission Creator](/documentation_v2/glossary.md#mission-creator)
runs on the [mission](/documentation_v2/glossary.md#mission) document: reading rows for its docks
and dialogs, placing and arranging entities, the [ORBAT](/documentation_v2/glossary.md#orbat) and
faction templates, loadouts, compositions, zones and tactical graphics. Each works on an explicit
`MissionDocCore` with plain values, and no UI type enters it.

## Contents

```text
apps/website/map-engine/src/data/store/operations/
├── apply_faction/        writes a faction library template onto one side's ORBAT, or refuses first
├── assets.rs             `PlacePayload`, and the object alias and class name of a resource name
├── attrs.rs              Attributes dialog reads, the multi-select diff, position and field writes
├── cargo.rs              loadout reads and writes, cargo seeding, the loadout buffer and Apply seed
├── cargo_rules.rs        `CargoRow`, the wear keys, and the cargo read, write and seed of a loadout
├── compositions.rs       saving a selection as a composition; composition rows, ids and entities
├── document_index.rs     the searchable index of every placed entity: kind, side, label, text
├── entity/               the per-kind entity operations and what a map release commits
├── environment.rs        `MissionEnv`: terrain, time, weather and map display fields read back
├── faction_library.rs    `FactionDoc`, `FactionRole`, `FactionVehicle`: the faction template shape
├── mod.rs                the module tree
├── place_orbat/          files a placed character into its side's current squad
├── placement/            the point math of the arrange commands and garrison firing positions
├── projections.rs        layer, faction, squad and slot rows read from the document
├── reassign.rs           the squad reassignment plan and its refusals, the move and the restore
├── rotation.rs           rotation snapping on the off, 5, 15 and 45 degree ladder, facing bearings
├── rows.rs               the plain row types the projections return
├── slot_ids/             the duplicate slot id check the save runs
├── tactical_graphics.rs  the tactical graphic draw, drag and selection session, and its row edits
├── tests/                unit tests for the cargo session state and the tactical draw machine
├── transform.rs          selection positions; pattern, align, space, orient and face-to commits
└── zones.rs              `DrawTarget` and `ZoneShape`, where a zone or trigger draw writes
```

## How it works

```text
Mission Creator gesture (apps/website/frontend/src/v2/apps/editor/)
   │
   ▼
crate::editing::hosted_commands: borrow the hosted MissionDocCore, pass host values
   │   (next_id, ensure_layer, confirm_bulk, the cargo seed)
   ▼
operations: read the document (small_maps_json, slots_json, materialize), plan in plain
   │   values, write through the MissionDocCore mutators
   ▼
the tail the host installed (crate::editing::history): rebind the map lanes, mark the mission dirty
```

An operation reads through the document's JSON views and `materialize`, decides in plain values,
and writes through the `MissionDocCore` mutators in `apps/website/map-engine/src/data/store/rows/`,
so every change keeps the store's field-level transactions and undo grouping. A decision only the
host can make arrives as a value or a callback that runs at the point it is needed: the folder a
placement files into, a bulk confirmation, the crew toggle, a zone's radius rule. The host keeps its
signals, dialogs and id counter; the pure math of `placement/` and `rotation.rs` never sees the
document at all.

Session state that outlives a call but is never saved lives here in thread-locals: the installed
cargo defaults, the loadout buffer and the seed each Apply draws from (`cargo.rs`), the tactical
graphics selection, draw and vertex drag (`tactical_graphics.rs`), and in `entity/` the two armed
drags and the layer-id counter. A vertex drag writes nothing until it is taken on release, so an
abandoned drag costs no undo step and a finished one costs one.

## Public surface

- `entity`: the entity operations, for `crate::editing::hosted_commands` and the Mission Creator's
  host state, input and outliner.
- `attrs`, `reassign`, `projections` and `rows`: the [slot](/documentation_v2/glossary.md#slot)
  attributes and squad reassignment commands, the Attributes dialog, the outliner and the editor
  context.
- `cargo` and `cargo_rules`: the slot loadout and ORBAT roster commands, and the
  [arsenal](/documentation_v2/glossary.md#arsenal) in
  `apps/website/frontend/src/v2/apps/editor/arsenal/`.
- `transform`, `rotation` and `placement`: the selection transform command, the canvas transform
  and, through `crate::editing::tools::placement`, the arrange menu.
- `document_index`, `compositions`, `zones` and `tactical_graphics`: the document search,
  composition library and zone authoring commands, the left dock, the zones panel and the tactical
  graphics bridge.
- `faction_library::FactionDoc` and `environment::MissionEnv`: re-exported as DTOs by
  `apps/website/frontend/src/v2/core/api/dto/`.
- `assets`: `PlacePayload` and the alias helpers, for the arsenal's asset catalog; `slot_ids`, for
  the Mission Creator's save.
- `apply_faction` and `place_orbat`: re-exported by `crate::data::store`.

## Boundaries

- Depends on: `crate::data::store` (`MissionDocCore` with its JSON views and mutators, `SlotSoa`,
  `NONE_IDX`, the stance codes, `EntityTransformPatch`); `crate::data::scenario` for the terrain
  bounds and the tactical graphic point limits; `serde` and `serde_json`.
- Used by:
  - `crate::editing`: `hosted_commands/`, `commands/merge_report.rs` and `tools/placement.rs`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/` (`arsenal/`, `bridge/`,
    `input/`, `mission_editor/`, `shell/` and `ui/`) and the DTOs of
    `apps/website/frontend/src/v2/core/api/dto/`;
  - `apps/website/map-engine/tests/operation_boundaries.rs` and the re-export pins in
    `apps/website/map-engine/src/data/store/tests/reexports.rs`.
- Rules:
  - a refused transform leaves the document and its undo depth unchanged, and a paste keeps unknown
    fields and the authored elevation (`refused_transform_leaves_document_and_history_unchanged`,
    `clipboard_paste_preserves_unknown_fields_and_authored_elevation` in
    `apps/website/map-engine/tests/operation_boundaries.rs`);
  - a Copy that finds nothing keeps the previous buffer, and every Apply draws a new seed
    (`a_copy_that_finds_nothing_leaves_the_previous_buffer_standing`,
    `every_apply_draws_a_seed_the_previous_apply_did_not` in `tests/cargo.rs`);
  - a tactical draw is armed only for a kind the validator gives a point floor, and a cancelled
    vertex drag writes nothing (`tests/tactical_graphics.rs`);
  - nothing here names a crate module outside `crate::data`, nor the graphics engine (rule 7 of
    `cargo xtask verify engine-layers`), and `cargo xtask verify editor-orbat-coherency` scans the
    files it lists here for `ensure_default_squad`.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the placement, arrange, loadout, zone and ORBAT features these operations serve.
