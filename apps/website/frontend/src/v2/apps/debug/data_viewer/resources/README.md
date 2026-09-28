# Equipment data viewer resources

The `resources` tab: the filtered resource catalog beside the selected resource's header and
sections.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/data_viewer/resources/
├── mod.rs               `Resources`: the resizable, searchable catalog filtered by domain and capability
└── resource_details.rs  `ResourceDetails`: identity, counts, downloads and the data, relationships, names
```

## Boundaries

- Depends on: the viewer's location, requests and layout in `apps/website/frontend/src/v2/apps/debug/data_viewer/`, the resource
  surface in `apps/website/frontend/src/v2/apps/debug/data_viewer/resource_data/`, the relationships section in `apps/website/frontend/src/v2/apps/debug/data_viewer/relationships/`, and
  `EquipmentResourcePage` in `apps/website/frontend/src/v2/core/api/dto/equipment_data_viewer/`.
- Used by: `page.rs` in `apps/website/frontend/src/v2/apps/debug/data_viewer/`, for the `resources` tab.
- Rules: the catalog stays mounted, with its scroll position remembered, while the selection
  changes (`equipment_viewer_catalog_filters_survive_resource_and_source_navigation` in
  `apps/website/frontend/src/v2/apps/debug/data_viewer/tests/navigation.rs`); a resource the selected generation lacks shows an absence notice
  instead of its sections.
