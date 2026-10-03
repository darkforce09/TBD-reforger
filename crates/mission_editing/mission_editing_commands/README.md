# Mission editing commands

The `mission_editing_commands` crate: the editing commands the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) runs over the live
[mission](/documentation/glossary/g_to_m.md#mission) document. Its hosted commands name what to
change, open the document the editing session hosts, commit one transaction through
`mission_operations` and run the post-change tail; its document texts are pure: the bytes an
export writes, the wording of the compile, merge and save reports, and the selection digests a
clipboard receives. It holds no browser or GPU code: the Mission Creator injects the folder a new
row is filed under, the confirmation a large move needs and the closed vocabularies as closures.

## Contents

```text
crates/mission_editing/mission_editing_commands/
├── Cargo.toml  the package: the editing session, the mission operations, document, model and validation, ids, layout tier 7
└── src/        the hosted commands, the document texts, the error and the crate root
```

## How it works

```text
Mission Creator (apps/frontend/src/workspaces/editor/), through `hosted_commands` and `document_text`
  │ a command with newtype ids (`impl Into<SlotUid>`, `impl Into<LayerId>`, ...) and values
  ▼
hosted_commands ── mission_editing_session::host::with_doc / with_host: one borrow
  │                mission_operations::<area> or a MissionDocCore mutator: one transaction
  │ borrow dropped
  ▼
mission_editing_session::history::after_local_edit, only when something changed
document_text ── pure over the compiled bytes, the findings, a merge report and the document JSON
```

Every id parameter is a newtype id taken as `impl Into<…>`, so a caller passes a `&str`, a `String`
or the typed id and the bytes the document stores stay the same: `SlotUid` (`orbat_slot_ids`) for
a slot's durable id; `SquadId`, `LayerId`, `CommentId`, `VehicleId`, `CrewSeatId`, `EntityId`,
`ConnectionId`, `CompositionId` and `FactionId` (`mission_document::ids`); `ZoneId`, `TriggerId`
and `MarkerId` (`mission_model::ids`); `AssetId` (`mission_validation`) for a slot's asset. A
selection digest row names its entity by `EntityId`. The faction apply and the compiled export
refuse with the crate's `Error`, whose `Display` text the Mission Creator shows verbatim. The
[source README](src/README.md) describes the modules.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_editing_commands   # export bytes, report wording, selection digests by value
```

## Public surface

- `hosted_commands`: the commands of `slot_attributes`, `selection_transform`,
  `composition_library`, `squad_reassignment`, `document_edit`, `document_search`,
  `entity_clipboard`, `orbat_roster`, `editor_layers`, `map_triggers`, `slot_loadouts`,
  `placed_vehicles`, `entity_connections`, `map_markers`, `zone_authoring` and `map_comments`, each
  re-exported at `hosted_commands::*`.
- `document_text`: `export_text` (`compiled_export_text`, `compile_diagnostics_summary`,
  `row_meta_missing_message`, `export_gesture_is_duplicate`, `apply_row_metadata_to_export`,
  `live_doc_title`), `merge_report` (`format_merge_report`, `duplicate_slot_id_report`),
  `selection_digest` (`SelectedEntity`, `resolve_selected_entities`, `prefab_leaf`,
  `entity_display_name`, `count_noun`, `format_grid_ref`, `grid_position_text`, `classnames_text`,
  `selection_summary_text`).
- `Error` and `Result`.
- `prelude`, which re-exports the most used of the items above.

## Boundaries

- Depends on: `mission_editing_session` (the host, the post-change tail, undo grouping),
  `mission_operations` (the authoring commands and row projections), `mission_document` (the
  document, the connection vocabulary, the document ids), `mission_model` (zone, trigger and
  marker ids), `mission_validation` (findings, `AssetId`), `formation_geometry` (the arrange
  vocabulary), `map_coordinates` (grid references), `orbat_slot_ids` (`SlotUid`), `serde_json`,
  `thiserror`.
- Used by: the Mission Creator in `apps/frontend/src/workspaces/editor/`, which imports
  `hosted_commands` and `document_text` directly.
- Rules: no `web_sys`, `leptos` or `wasm_bindgen` in this crate; the place path never calls
  `ensure_default_squad` (`cargo xtask verify editor-orbat-coherency` scans every hosted command);
  every public id parameter and field is a newtype id (`cargo xtask verify crate-anatomy`);
  mission editing tier 7 (`cargo xtask verify crate-tiers`).

## Related documentation

- [Mission editing crates](/crates/mission_editing/README.md) — the category and its crates.
- [Mission editing session](/crates/mission_editing/mission_editing_session/README.md) — the host
  and the post-change tail these commands run through.
- [Mission operations](/crates/mission/mission_operations/README.md) — the document operations the
  hosted commands drive.
- [Editing layer](/documentation/crates/mission_editing/editing_layer.md) — the host, hosted commands,
  undo and tools as flows.
