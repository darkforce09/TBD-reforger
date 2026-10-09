# Equipment data viewer

The `/debug/data-viewer` bench: a full-window, URL-only reader of the equipment and vehicle
datasets the API imports from the published Workbench exports. It browses the gameplay catalog
or the full diagnostics, one resource's native containers and fields, relationships, the field
inventory, the gameplay selection decisions and the export history.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/data_viewer/
├── browsing_state.rs    per-session memory of expanded values and catalog positions, bounded
├── data/                cancellable, bounded API reads and the page cache
├── field_inventory/     the `fields` tab: native field combinations and their occurrences
├── layout/              panels, pagers, search boxes, feedback and the virtual list
├── mod.rs               the module tree; `DataViewerPage` and the panels are browser-only
├── navigation_state.rs  the shareable location: dataset, generation, tab, resource, section, cursors
├── overview/            the overview, selection and export history tabs
├── page.rs              `DataViewerPage`: status polling, generation choice, tab bar, active tab
├── relationships/       forward and reverse occurrences of a resource
├── resource_data/       one scrolling surface of every native container of a resource
├── resources/           the filtered resource catalog and the selected resource's header
├── source_inspector/    native value and provenance rendering, and the original document view
└── viewer.css           the viewer's own styles, inlined by `page.rs`
```

## How it works

```text
URL query ─▶ Navigation (navigation_state.rs) ─▶ page.rs: status poll every 5 s ─▶ generation
        │                                                      │
        ▼                                                      ▼
tab: overview · resources · fields · selection · generations · document
        │
        ▼
data/ loading ─▶ public_get /api/v1/debug/equipment-data/<endpoint>?dataset=…&generation=…
             ─▶ equipment_data_viewer DTOs ─▶ tab components
```

Every viewer state lives in the URL, so a location can be shared and the back button restores
it; native node identifiers stay opaque. `page.rs` polls `status` every five seconds while the
browser tab is visible, follows a newly published generation when the location says `latest`,
shows import progress, and mounts the tab the location names. The dataset switch moves between
the gameplay catalog and the diagnostics. Reads go through `data/`, which aborts a superseded read
so it cannot overwrite a newer location, and caches settled pages per generation.
`browsing_state.rs` keeps expanded values and catalog positions for recently viewed resources
only. The panels compile for `wasm32` alone; `navigation_state.rs` and `browsing_state.rs` are
pure.

## Public surface

- `DataViewerPage`: the route component `crates/frontend/shell/frontend_application/src/app_routes.rs` mounts at
  `/debug/data-viewer`.
- `navigation_state` and `browsing_state`: the pure location and browsing-memory models.

## Boundaries

- Depends on: `frontend_transport::client::public_reads::public_get` and the DTOs in
  `crates/frontend/foundation/frontend_api_dtos/src/equipment_data_viewer/`; `leptos`, `web_sys`,
  `gloo_timers` and `serde_json`.
- Used by: the `/debug/data-viewer` route in `crates/frontend/shell/frontend_application/src/app_routes.rs`, with its
  row in `crates/frontend/foundation/frontend_route_table/src/routes.rs` (full-bleed, chromeless, route tier `none`); the live
  check `gate equipment-data-viewer` in
  `tools/browser_testing/browser_gate_suites/src/equipment_data_viewer/`.
- Rules: the viewer only reads, anonymously, and never touches the session; a location keeps
  exact source identities; a dataset switch drops dataset-specific documents; the browsing memory
  keeps expansions and evicts old resources.

## Related documentation

- [Equipment and vehicle source export](/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md)
  — the Workbench exporter that produces the datasets the viewer reads.
