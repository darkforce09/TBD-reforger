**Status:** live

# Performance at scale

How the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) behaves when a
[mission](/documentation_v2/glossary/g_to_m.md#mission) holds tens of thousands of
[slots](/documentation_v2/glossary/n_to_z.md#slot): bulk paste, the windowed trees, the clustered map at
far zoom, the drag and pick paths, the per-edit redraw, and the load and save of a large
document. Each entry states what the code does; none records a measured frame rate or duration.

## Where it lives

- Code: the slot icon lane, its selection patches, drag overlay and clusters in
  [`apps/website/map-engine/src/overlay/symbology/instances/`](/apps/website/map-engine/src/overlay/symbology/instances/README.md);
  the picks in `apps/website/map-engine/src/editing/picking.rs` and
  `apps/website/map-engine/src/spatial/indexing/picking.rs`; the windowed trees in
  [`apps/website/frontend/src/v2/apps/editor/ui/outliner/tree/`](/apps/website/frontend/src/v2/apps/editor/ui/outliner/tree/README.md)
  and `apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager/tree_panel.rs`; the undo
  driver's redraw in `apps/website/frontend/src/v2/apps/editor/bridge/document_host/history.rs`;
  the load in [`apps/website/frontend/src/v2/apps/editor/shell/hydrate/`](/apps/website/frontend/src/v2/apps/editor/shell/hydrate/README.md)
  and the save in [`apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/`](/apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/README.md).
- Related features: [editor route and boot loading](/documentation_v2/website/frontend/apps/editor/feature_inventory/editor_route_loading.md)
  (the boot overlay), [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md),
  [selection](/documentation_v2/website/frontend/apps/editor/feature_inventory/selection.md),
  [transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| PERF-BULK-PASTE-001 | Bulk paste as one transaction | shipped |
| PERF-OUTLINER-001 | Windowed layer and ORBAT trees | shipped |
| PERF-CLUSTER-001 | Cluster discs at far zoom | shipped |
| PERF-WORKER-001 | Compile off the main thread | not built |
| PERF-CHUNK-001 | Slots held in spatial chunks | not built |
| PERF-LOAD-001 | Load with determinate progress | partial |
| PERF-SAVE-001 | Save a large version with progress | partial |
| PERF-DRAG-001 | Drag preview without document writes | shipped |
| PERF-PICK-001 | Spatial index for click and marquee | partial |
| PERF-BIND-001 | Incremental redraw after an edit | partial |
| PERF-SESSION-001 | Warm return without a server read | partial |
| PERF-IDB-001 | Chunked local-draft restore with progress | not built |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).

### PERF-BULK-PASTE-001 — Bulk paste

Ctrl/Cmd+V writes every copied slot in one document transaction, one undo step, and selects every
pasted slot; no cap limits the count or the selection (`paste_slots`,
`apps/website/map-engine/src/data/store/rows/paste.rs:22-55`; KEY-COPY-001). A bulk delete is one
`remove_slots` call (XFORM-DEL-001).

### PERF-OUTLINER-001 — Windowed trees

1. The layers tree draws every row up to 50 (`VIRTUAL_SLOT_THRESHOLD`,
   `ui/outliner/outliner.rs:27`); above that it draws only the visible 16 px rows plus six of
   overscan between two spacers (`ui/outliner/tree/row_geometry.rs:197-201`), and publishes its
   counts to `window.__outlinerStats`.
2. The ORBAT Manager's tree windows the same way above the same threshold, with 32 px rows and
   eight of overscan (`ui/modals/orbat_manager.rs:27-29`, `orbat_manager/tree_panel.rs:71`).

### PERF-CLUSTER-001 — Clusters

With more than 500 slots (`CLUSTER_SLOT_THRESHOLD`) at zoom −4 or farther (`ZOOM_CLUSTER_MAX`),
the slot lane switches to discs sized by how many slots each covers; the camera re-checks the gate
on every move (`cluster_mode`, `apps/website/map-engine/src/overlay/symbology/instances/symbols.rs:44-56`;
`apps/website/map-engine/src/camera/viewport.rs:110-128`). The cluster grid is built over Everon's
bounds whatever the terrain (`instances/bridge_1.rs:228-241`). Clicking a disc does not zoom
into it; picks still test the slots beneath.

### PERF-WORKER-001 — Compile worker

Not built: Save Version, both exports, the conflict check and the validation chip compile the
document on the page's main thread (`compile_payload` in `shell/document_commands/imp/mission_saving.rs:51`;
DATA-COMP-001).

### PERF-CHUNK-001 — Slot chunks

Not built: the document keeps slots in one map, and the icon lane holds every slot at once. The
map engine's 512 m chunks stream terrain and world objects only.

### PERF-LOAD-001 — Load

1. The boot overlay's "Loading mission…" segment streams the saved version and advances by bytes
   against the response's `content-length` (`shell/hydrate/server_reconciliation.rs:25-75`); the
   terrain, satellite and world-object segments advance by their own budgets (FILE-BOOT-001).
2. Partial: the local draft is read as one blob with no progress of its own, and every slot is
   bound to the icon lane in one pass once the document is ready.

### PERF-SAVE-001 — Save

1. The Save Version dialog shows the estimated size before saving, yellow over 200 MB, and an
   indeterminate bar while the status reads "Saving v…" (`ui/docks/top_strip/view/overlays.rs:28-51`,
   `:119-130`).
2. The client compiles and posts the whole payload; the server lifts its body limit for this route
   alone, 256 MiB by default, and answers 413 above it ("Payload too large").
3. Partial: the bar shows activity, not progress, and the compile runs on the main thread
   (PERF-WORKER-001).

### PERF-DRAG-001 — Drag

A drag uploads the dragged rows once into an overlay, hides them in the base lane and then moves
them with a shader offset; the document is written once, on release (`set_drag` and
`drag.rs` in `overlay/symbology/instances/`; XFORM-MOVE-001).

### PERF-PICK-001 — Picks

1. Click, drag-start, right-click, double-click and marquee picks query a grid point index over
   the slot positions (`pick_slot_row`, `apps/website/map-engine/src/spatial/indexing/picking.rs:9-24`),
   and vehicles are tested in a separate pass.
2. Partial: every pick first rebuilds the slot table from the whole document
   (`map_render_slot_soa`, `apps/website/map-engine/src/editing/selection_universe.rs:107-111`)
   and then builds the index afresh, so a single click costs work in proportion to every slot.

### PERF-BIND-001 — Redraw after an edit

1. A selection change patches only the icon rows that changed, 12 bytes each, and never repacks
   the lane (`set_selection`, `overlay/symbology/instances/patches.rs`).
2. Partial: every document edit runs the undo driver's tail, which rebuilds the slot table and
   rebinds the whole slot lane, the squad links and the vehicle lane
   (`after_doc_change`, `bridge/document_host/history.rs:271-300`).

### PERF-SESSION-001 — Warm return

1. Switching away from the tab and back never reloads the page, so the document stays in memory.
2. Partial: a reload always restores the draft and fetches the server version again. The
   warm-session marker is written after each boot (`mark_ready`, `shell/session.rs:56`), but only
   the `__missionPersist.warm()` test probe reads it (`read_warm`, `shell/session.rs:80`).

### PERF-IDB-001 — Local draft restore

Not built: the draft is one whole-document CRDT update in the IndexedDB store `tbd-mission-yrs`,
read and applied in one step (DATA-IDB-001); there is no per-chunk restore and no restore
progress.

### Known discrepancies

- None found: the READMEs of these folders agree with the code on every mechanism above.

## Data

- `GET /api/v1/missions/{id}` (streamed, measured by `content-length`) at boot and
  `POST /api/v1/missions/{id}/versions` on save; both are described in
  [data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md#data).

## Design

- Design target: the scale expectations of the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md)
  and the [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md).
  The inventory records the mechanisms; measured frame rates belong to the engine benchmarks.

## Open work

- [T-938 — Engine and wasm performance](/documentation_v2/tickets/specs/t938_engine_perf.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-938_plan.md)): pooled lane buffers, GPU
  culling for every icon lane and a wasm memory guard.
- [T-939.7 — Vehicles panel virtualization, memoized outliner flatten](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_7_plan.md)): windowed vehicle rows and a
  cached tree flatten.
- [T-140 — Mission client payload budget](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-140_plan.md)): a size budget the compile
  reports against.
- [T-734 — Validation panel: full-compile cost, wasm panic, seam pin](/.ai/tickets/T-734.toml),
  [T-729 — Owner-line materialize per frame; zones mislabels triggers](/.ai/tickets/T-729.toml),
  [T-731 — ROW_ACTIVE border-t skews virtual tree by 1px](/.ai/tickets/T-731.toml) and
  [T-847 — push_drag_preview Class-R pins omit bind_squad_link_preview](/.ai/tickets/T-847.toml)
  (deferred, no plan).

No open ticket covers the per-pick index rebuild, the full slot rebind after every edit or a
compile worker.

## Decisions

- A drag never writes the document until release: the preview is a GPU offset, so a drag's cost
  does not grow with the mission.
- A selection change patches rows rather than repacking the lane: selecting stays cheap at any
  mission size.
