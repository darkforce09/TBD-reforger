# Hosted editor commands

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s document commands that run
against the installed editing host: each names what to change, opens the hosted
[mission](/documentation/glossary/g_to_m.md#mission) document, calls the matching
`mission_operations` edit, and runs the post-change tail. A caller passes no document
handle, selection set or undo bookkeeping.

## Contents

```text
crates/mission_editing/mission_editing_commands/src/hosted_commands/
├── composition_library.rs  save a selection as a composition; rename, recategorise, credit, delete
├── document_edit.rs        `commit_document_edit`: run one caller-supplied mutator, then the tail
├── document_search.rs      the searchable index of every placed entity, and of the selected ones
├── editor_layers.rs        folder tree: create, rename, delete, reparent, refile, hide, lock, drag
├── entity_clipboard.rs     copy, paste at the cursor and delete over the selection
├── entity_connections.rs   arm and complete a connection, list and check them, force a formation
├── map_comments.rs         map comments: place, edit, duplicate, refile, delete
├── map_markers.rs          a faction's briefing markers: icon, caption, position, removal
├── map_triggers.rs         the authored triggers, their activation, rules and owner edge
├── mod.rs                  the module tree; re-exports every command and row type
├── orbat_roster.rs         ORBAT roster: squads, slots, leaders, vehicles, refile, factions
├── placed_vehicles.rs      placed vehicles: cargo, heading, crew seats and positions
├── selection_transform.rs  align, space, orient, pattern and rotate-to-face the selection
├── slot_attributes.rs      Attributes panel reads and commits, one slot or the selection
├── slot_loadouts.rs        slot loadouts: read, seed, buffer a selection, commit writes
├── squad_reassignment.rs   move a selection into another faction's squad, and back
└── zone_authoring.rs       the authored zones: type, label, faction, rules, add and delete
```

## How it works

```text
host UI ──► hosted command (newtype ids, values, host closures)
               │ mission_editing_session::host: one borrow of the document (selection, id minter)
               ▼
            mission_operations::<area>  one transaction, or one group for a set
               │ borrow dropped
               ▼
            mission_editing_session::history::after_local_edit  only when something changed
```

Every entry point opens one host borrow and drops it before the tail, because the tail reads the
same document. A command that changed nothing runs no tail, and a command over many rows runs one
tail for the whole set, so an apply-to-all or a batch reassign is one undo step
(`squad_reassignment.rs` brackets its moves with `mission_editing_session::batch::with_batch`).
The exceptions are `commit_document_edit` and the composition edits (rename, recategorise, credit,
delete), whose mutators return nothing: they run the tail whenever a document is hosted.

Every id a command takes is a newtype id passed as `impl Into<…>`, so a caller hands a `&str`, a
`String` or the typed id: `SlotUid` for a slot, `SquadId`, `LayerId`, `CommentId`, `VehicleId`,
`CrewSeatId`, `EntityId`, `ConnectionId`, `CompositionId` and `FactionId` from `mission_document`,
and `ZoneId`, `TriggerId` and `MarkerId` from `mission_model`. `orbat_apply_faction` refuses with
the crate's `Error`, whose text the caller renders verbatim.

What only the host knows crosses as a closure: the folder a new entity is filed under
(`ensure_layer`), the confirmation a large move needs (`confirm_bulk`), and the closed vocabularies
of marker icons and zone types, so a value outside the schema is never stored. New ids come from
the host's per-session counter (`EditingHost::next_id`, starting at 0 on each install), passed to
the document's minting operations. Interaction state that is not document content stays in this
folder's thread-locals: the copied rows of `entity_clipboard.rs` and the armed connection of
`entity_connections.rs`, neither of which is an undo step.

## Boundaries

- Depends on: `mission_editing_session::host` (the document, selection and id minter),
  `mission_editing_session::history::after_local_edit`, `mission_editing_session::batch`,
  `formation_geometry` for the arrange vocabulary, `mission_document` (`MissionDocCore`,
  `ConnectionKind`, the document ids), `mission_model` and `orbat_slot_ids` (ids),
  `mission_validation` (`AssetId`), and the `mission_operations` modules for attributes, cargo,
  compositions, document index, entities, faction library, projections, reassignment, transforms
  and zones.
- Used by:
  - the Mission Creator in `crates/frontend/workspaces/mission_creator_workspace/src/`, directly: the bridge's host
    state,
    armed placement, overlays and document history, the pointer input, the arsenal, the docks
    (context menu, left and right docks, markers, toolbelt, top strip), the inspector (Attributes
    modal, audio emitters, zones panel), the modals, the outliner, and the editor tests; the
    pointer input commits the `map_editing_tools` `Rotate` gesture through
    `selection_transform::rotate_selection_to_face`.
- Rules: the document borrow ends before the tail runs; the place path in these
  files never calls `ensure_default_squad`; every id parameter is a newtype id (`cargo xtask verify crate-anatomy`); no
  `web_sys`, `leptos` or `wasm_bindgen`.

## Related documentation

- [Mission document](/crates/mission/mission_document/README.md) — the document and
  the operations these commands drive.
- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — the [ORBAT](/documentation/glossary/n_to_z.md#orbat), layers, markers, zones, triggers,
  connections and clipboard features.
- [Mission Creator feature inventory: transform and delete](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/transform_and_delete.md) — the arrange, rotate, formation and delete commands.
