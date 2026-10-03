# Mission editing session

The `mission_editing_session` crate: the editing session the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) runs over the live
[mission](/documentation/glossary/g_to_m.md#mission) document. It holds the installed editing host
and the one borrow chain every editing command reaches the document by, steps the document's own
undo stack, groups several transactions into one undo step, routes a subject id to the selection
surface that owns it, answers which ids a selection may name, joins spatial picks to document ids
and draws the editor-authored overlay lanes (connections, comments, briefing markers). It holds no
browser or GPU code: the Mission Creator injects every clock, prompt and post-change hook.

## Contents

```text
crates/mission_editing/mission_editing_session/
├── Cargo.toml  the package: the mission document, slot columns, subject id, camera, point picks and symbology, layout tier 6
└── src/        the host, the undo drive, grouping, routing, the selection universe, the picks and the lanes
```

## How it works

```text
Mission Creator (apps/frontend/src/workspaces/editor/), which links mission_editing_session directly
  │ creates the MissionDocCore handle and the selection, then host::install(doc, selection)
  ▼
host ── with_doc / with_doc_mut: one borrow per call, dropped before returning
  ├─ history ──────────── MissionDocCore::undo / redo, then the host's after_document_change hook
  ├─ batch ────────────── begin_group / end_group around a multi-transaction edit (drop guard)
  └─ routing, selection_universe, picking, lanes ── read the document's JSON and slot columns
hosted commands, map tools (map engine) and mission_persistence build on host and history
```

The host is one thread-local `EditingHost`: the document handle (`DocHandle`, the same shared cell a
restore or a hydrate swaps into, so a command always sees the live document), the selected ids and
a per-session id counter that restarts at 0 on every install. Every committed edit ends in
`history::after_local_edit`, the one hook the host installed (prune the selection, rebind lanes,
mark unsaved, schedule a save); undo and redo end in the same hook after their mutable borrow is
dropped. The picks turn a frozen camera and a pixel into a world query, ask
`spatial_indexes::point_indexes::picking` for rows, and let `MissionDocCore` map rows to ids and
break ties (a [slot](/documentation/glossary/n_to_z.md#slot) beats a vehicle at equal distance).
`selection_universe` reads membership from the post-change document's raw maps rather than the
materialized slots, so hiding a slot never deselects it, and `routing` answers the affordance
probe and the click with one resolution, so a row is clickable only when a click reaches something.
The [source README](src/README.md) describes the modules.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_editing_session   # picks, marquees, the tie policy and squad link inputs
```

## Public surface

- `host`: `install`, `with_host`, `with_doc`, `with_doc_mut`, `doc_handle`, the selection reads and
  setters, `retain_selected`, `EditingHost`, `DocHandle`, `SelectionHandle`.
- `history`: `undo`, `redo`, `after_local_edit`, `HistoryHost`, `install_host`.
- `batch::with_batch`.
- `routing`: `route_target` (a subject as `mission_validation::SubjectId`), `route_availability`,
  `RouteTarget`.
- `selection_universe`: `selectable_ids`, `crewed_slot_ids`, `map_render_keep_indices`,
  `map_render_slot_soa`, `filter_slot_soa_excluding`, `plain_paste_anchor`.
- `picking`: `pick_slot`, `pick_slot_or_vehicle`, `marquee_slot_ids`, `marquee_ids_with_vehicles`,
  `squad_link_inputs`.
- `lanes`: `comments` (`CommentPoint` keyed by `mission_document::ids::CommentId`), `connections`
  (`ConnSegment` keyed by `ConnectionId`), `markers::marker_lane_fields`.
- `prelude`, which re-exports the most used of the items above.

## Boundaries

- Depends on: `mission_document` (`MissionDocCore`, its undo and grouping, its tie policy and its
  ids), `mission_crdt` (`SlotSoa`), `mission_validation` (`SubjectId`), `camera_math` (the frozen
  orthographic camera), `spatial_indexes` (point picks and marquees), `unit_symbology` (squad link
  inputs, side tints), `serde_json`; dev `mission_operations` (the ORBAT placement of a pick test).
- Used by: `mission_persistence` (`host::DocHandle`), `mission_editing_commands` (the host and
  the post-change tail), `map_editing_tools` (the picks) and the Mission Creator in
  `apps/frontend/src/workspaces/editor/`.
- Rules: no `web_sys`, `leptos` or `wasm_bindgen` in this crate; picks are square for slots and
  circular for vehicles, ties go to the slot and marquees list slots before vehicles
  (`square_slots_circular_vehicles_and_equal_distance_policy`); mission editing tier 6
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Mission editing crates](/crates/mission_editing/README.md) — the category and its crates.
- [Mission document](/crates/mission/mission_document/README.md) — the document this session
  hosts.
- [Editing layer](/documentation/crates/mission_editing/editing_layer.md) — the host, hosted commands,
  undo and tools as flows.
