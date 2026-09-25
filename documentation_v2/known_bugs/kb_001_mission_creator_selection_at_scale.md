**Status:** live

# KB-001 — Selection and copy-paste break at extreme slot counts in the Mission Creator

## Status

Resolved: not reproducible from the code. The defect was seen in the React
[Mission Creator](/documentation_v2/glossary.md#mission-creator), whose whole tree, the selection,
picking and icon code it names included, commit `50bba633e` deleted when the Leptos app became the
only frontend. The Rust selection pipeline that replaced it shares no code with it, and no run of
it has shown the symptom. Severity low while it lasted: it appeared only far above the realistic
envelope. Real missions stay well under about 10,000 [slots](/documentation_v2/glossary.md#slot),
and the defect showed at 517,968 objects. Area: Mission Creator selection, picking and slot icon
drawing.

## Symptom

Seen once, in an operator browser gate of the Rust document core with 517,968 objects, a document
built by in-session copy and paste rather than a reload, in the detail render mode (zoom about
-3.40; the cluster mode engaged only at `ZOOM_CLUSTER_MAX` = -4 or below). Frame rate held at
60 fps, so it was not a performance defect.

1. Ghost selection: after a select and a deselect, some objects stayed highlighted while the
   toolbelt read `SEL 0`. The store's `selection.ids` was empty while the icon cache's `.selected`
   flags, or the drawn colours, had not cleared.
2. The ghost-highlighted objects could not be selected by a click.
3. Ctrl+C and Ctrl+V did nothing. The copy handler read `selection.ids`, so with the ghost state's
   empty selection the in-editor clipboard stayed empty and a paste had nothing to place.

## Cause

Never fully root-caused. The selection, picking and drawing code was unchanged by the document-core
change under test, so the gate exposed a scale limit of that code rather than a regression. The
change's own parity test proved the store dictionaries byte-identical to the Rust document, but it
never compared the icon cache, the spatial index or the cluster index with them.

The leading suspect was the uncapped marquee: the paste path capped its post-paste selection at
`BULK_SELECT_CAP` (500), because ten thousand selected ids overloaded the highlight set and the
outliner, while the marquee release in `useSelectTool` put every id `slotSpatialIndex.pickRect`
returned into `selection.ids`. A six-figure selection then stressed `setSelectionFlags` (a pass over
every icon), the virtual outliner and the colour attribute of the map layer. It stayed a hypothesis.

The code the entry describes no longer exists: `git ls-files apps/website/frontend/src/features`
lists nothing. In the Rust Mission Creator, the marquee release and a paste still select every id
they reach with no cap (`apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/pointer_up.rs`,
`paste_at_cursor` in `apps/website/map-engine/src/editing/hosted_commands/entity_clipboard.rs`), and
a selection change patches only the icon rows whose selected state flips (`set_selection` in
`apps/website/map-engine/src/overlay/symbology/instances/bridge_1.rs`). The
[performance at scale](/documentation_v2/website/frontend/apps/editor/feature_inventory/performance_at_scale.md)
inventory holds what the Rust pipeline does at scale.

## Workaround

None needed.

## Fix

No change fixed it; deleting the React frontend removed the code it lived in. If the symptom is
seen again in the Rust Mission Creator, reproduce it before touching code: seed about 500,000
slots, marquee-select, deselect, and compare the selected ids with the icon rows the engine holds
as selected; a mismatch is a state defect a test can pin, while a match with a stale picture is a
drawing defect that needs the browser. A new entry records it; this one stays resolved.

## Related tickets

- [T-059 — Bulk paste/delete at scale](/documentation_v2/tickets/specs/t059_bulk_paste_operations.md)
  (shipped): the bulk paste and delete, and the 500-id post-paste selection cap of the React code.
- [T-065 — Cluster LOD at extreme zoom](/documentation_v2/tickets/specs/t065_cluster_lod.md)
  (shipped): the cluster mode below zoom -4.
- [T-067 — Spatial chunks](/documentation_v2/tickets/specs/t067_spatial_chunks.md) (shipped): the
  chunk cull of the React map layers.
- [T-145 — Rust/Wasm Doc Core (Yjs replacement)](/.ai/tickets/T-145.toml) (shipped): the document
  core change whose gate exposed the defect.
- [T-090 — Map visualization program](/documentation_v2/tickets/specs/t090_091_map_terrain_program.md)
  (ready): the map and scale program the gate ran under.
