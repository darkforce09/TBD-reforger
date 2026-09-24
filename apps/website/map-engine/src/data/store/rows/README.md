# Mission document core

`MissionDocCore`, the `yrs` document that holds one open
[mission](/documentation_v2/glossary.md#mission) while the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) edits it: its root maps, every
field-level write, the JSON views the compiler and the docks read, the
[slot](/documentation_v2/glossary.md#slot) projection, load, merge and paste, peer updates and undo.

## Contents

```text
apps/website/map-engine/src/data/store/rows/
├── briefings.rs         faction briefing markers and briefing prose
├── comment_rows.rs      comment row reads: the map, the position and the text fields
├── comments.rs          map comments: add, edit, duplicate, remove, layer filing, JSON view, seed
├── compositions.rs      saved compositions: add, edit, remove, JSON view, and their map stamp
├── connection_types.rs  the connection kinds, rows and findings, and `validate_connection_rows`
├── connections.rs       connections: add, remove, remove all touching an id, JSON and findings
├── construction.rs      `new`, `with_client_id`, `with_undo_clock`: the root maps, the undo manager
├── crew_rows.rs         a vehicle's crew seat map, read and written
├── doc_core.rs          the `MissionDocCore` fields, `SquadMembership`, `EntityTransformPatch`
├── entities.rs          world objects: add, set faction, remove
├── formation_rows.rs    `formation_offsets` and `FORMATION_SPACING_M`: each formation's offsets
├── formations.rs        `force_to_formation`: a squad's members re-formed around its leader
├── hydrate.rs           `hydrate`: the document replaced by a saved editor payload
├── json_rows.rs         JSON and `Any` conversion, rows loaded in authored order, typed field reads
├── layer_rows.rs        own and inherited layer flags, a slot's layer, the transform lock, detach
├── layers.rs            layer folders: add, rename, hide, lock, reparent, remove, file a slot
├── lookup.rs            client id, origin mode, peer updates, the JSON views and slot lookups
├── marker_rows.rs       briefing marker rows, and markers parked for a faction not yet created
├── materialize.rs       `materialize`: the visible slots as a `SlotSoa`
├── merge.rs             `merge_mission_payload`: another mission merged in, with ids reminted
├── merge_index.rs       the resident factions, squads and ids a merge matches against
├── merge_json.rs        `merge_mission_payload_json`: the merge with JSON in and a JSON report out
├── merge_report.rs      `MergeOpts` and `MergeReport`: the merge offset and what the merge did
├── metadata.rs          title, environment and row metadata (terrain, time, weather, briefing)
├── mod.rs               the module tree; re-exports `MissionDocCore` and the types its methods take
├── paste.rs             `paste_slots`: copied slots written at the cursor in one transaction
├── position_rows.rs     position and shape values, and the moves slots, vehicles and objects share
├── remint.rs            `RemintMap`: incoming ids mapped to resident or fresh ones
├── roster.rs            the ORBAT roster: slots, factions, squads, leaders, order, squad moves
├── side_cache.rs        `SideKeyMemo`: squad side keys cached until the next transaction
├── slot_edits.rs        slot field, identity, object, loadout and batch edits, removal, a seed
├── slot_rows.rs         in-transaction slot writes: the leader rule, squad clean-up, indices
├── tests/               unit tests for writes, undo steps, peers, hydrate and compile, merges
├── transforms.rs        slot, vehicle and object moves, rotations and transform patches
├── triggers.rs          triggers: add, reshape, name, owner, activation, rules, remove, JSON view
├── undo.rs              undo, redo, explicit undo groups and the depth cap
├── vehicles.rs          vehicles: add, faction, cargo, attach and detach, crew seats, remove
└── zones.rs             zones: add, reshape, type, label, faction, rules, remove, JSON view
```

## How it works

One `MissionDocCore` wraps one `yrs` `Doc` with its own client id: `new` mints a random one and
`with_client_id` takes a fixed 53-bit id. Its root maps hold one row per id:

```text
slots  squads  factions  editorLayers        the ORBAT and the layer tree; slotIds, entityIds native
vehicles  entities  zones  triggers  markers  comments  connections  compositions
loadouts  items  objectives  meta             meta: title, terrain, map, environment, parked markers
entityOrder  payloadExtras                     authored row order, and payload keys kept as sent
```

Every mutator opens one transaction through `begin`, under the `local-user` origin, or under `init`
while `set_origin_init(true)` is on. The undo manager covers every map but `entityOrder` and
`payloadExtras` and tracks only `local-user`, so a hydrate, a seed or a restore run in init mode is
never an undo step, and a batch write (a mixed move, a paste, a merge, a batch attribute edit) is
exactly one; the window, clock and cap come from
`apps/website/map-engine/src/data/store/crdt/undo_groups/`. `apply_update` applies a peer's update
under `init`, so undo takes back only local edits, and it refuses an update that carries more edits
under the document's own client id than the document has made.

`small_maps_json` returns every map but `slots` as `…ById` JSON, with `meta`, `entityOrder` and
`payloadExtras`, where zones, compositions, triggers, comments and connections are repeated in
authored order; `slots_json` returns the slot rows with exact `f64` positions. Together they are the
whole document the compiler in `crate::data::scenario` reads. `materialize` projects the visible
slots into a `SlotSoa`, and resolves each squad's side key once through `SideKeyMemo`, which an
after-transaction observer on the document clears.

`hydrate` clears every row map but `connections`, replaces the `meta` keys the payload carries,
loads the payload's rows in authored order, keeps unknown top-level keys in `payloadExtras`,
promotes plain `slotIds` and `entityIds` to native arrays and adds a default layer when none
arrived. `merge_mission_payload` adds another mission in one transaction: factions fold by name and
side key, squads by name and side, other colliding ids are reminted, an optional offset moves every
placed entity, and malformed rows land in `MergeReport.skipped`; comments, connections and loadouts
are not merged.

Every slot write keeps the roster whole: a squad's leader is one of its slots (the first when the
leader leaves), a squad left without slots is removed with its vehicles, and slot indices follow
`slotIds`. `update_slot_position` clamps x and y to the terrain size the caller passes, and it and
`move_entities` skip a slot whose layer, or one above it, is locked. A briefing marker for a faction
that does not exist yet is parked in `meta` and moved into the faction's briefing when the faction
is added.

## Boundaries

- Depends on: `crate::data::store::crdt` (the id arrays, `SlotSoa` and its interner, the undo
  groups); `yrs` (`Doc`, maps, arrays, `UndoManager`, updates, the after-transaction observer);
  `serde_json`.
- Used by:
  - `crate::data::store`, which re-exports `MissionDocCore`, `EntityTransformPatch`,
    `SquadMembership`, the connection types, `validate_connection_rows` and `formation_offsets`,
    and its `operations` and `selection` modules, which work on the document;
  - `crate::editing` (the host, history, hosted commands, persistence, picking, lanes and tools),
    and the store-gated tests in `apps/website/map-engine/src/data/scenario/compiler/` and
    `apps/website/map-engine/tests/`;
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/`: the document host, editor
    context and overlays in `bridge/`, the canvas mount in `mission_editor/`, the hydrate,
    persistence and document commands in `shell/`, and the inspector, the settings dialog and the
    outliner in `ui/`.
- Rules:
  - a local write is one undo step and an init-mode write is none
    (`two_local_places_are_two_undo_steps`, `init_mode_transactions_are_not_undoable` in
    `tests/cases_2.rs`); a batch is one step
    (`update_slots_attr_batch_is_one_undo_step_across_many_slots` in `tests/cases_1.rs`,
    `merge_is_one_undo_step_and_undo_restores_exactly` in `tests/cases_6.rs`);
  - peers with distinct ids merge and a colliding id is refused
    (`two_peers_with_distinct_ids_merge_concurrent_edits`,
    `colliding_client_ids_are_rejected_not_merged` in `tests/cases_2.rs`);
  - a hydrate and a compile keep unknown top-level keys, the schema, the map and the slot order
    (`unknown_top_level_keys_survive_compile_hydrate_compile` in `tests/cases_2.rs`,
    `t220_hydrate_compile_preserves_schema_map_and_slot_order` in `tests/cases_3.rs`);
  - `cargo xtask verify editor-orbat-coherency` runs `set_leader_exclusive`,
    `empty_squad_garbage_collected`, `move_slot_bidirectional`, `leader_invariant_holds` and
    `attach_vehicle_roundtrip` from `tests/cases_2.rs`.
