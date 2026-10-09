# Equipment data viewer resources

The `resources` tab: the filtered resource catalog beside the selected resource's header and
sections.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/data_viewer/resources/
├── mod.rs               `Resources`: the resizable, searchable catalog filtered by domain and capability
└── resource_details.rs  `ResourceDetails`: identity, counts, downloads and the data, relationships, names
```

## Boundaries

- Depends on: the viewer's location, requests and layout in `crates/frontend/workspaces/debug_benches/src/data_viewer/`, the resource
  surface in `crates/frontend/workspaces/debug_benches/src/data_viewer/resource_data/`, the relationships section in `crates/frontend/workspaces/debug_benches/src/data_viewer/relationships/`, and
  `EquipmentResourcePage` in `crates/frontend/foundation/frontend_api_dtos/src/equipment_data_viewer/`.
- Used by: `page.rs` in `crates/frontend/workspaces/debug_benches/src/data_viewer/`, for the `resources` tab.
- Rules: the catalog stays mounted, with its scroll position remembered, while the selection
  changes; a resource the selected generation lacks shows an absence notice
  instead of its sections.
