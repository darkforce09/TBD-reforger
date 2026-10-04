# Equipment data viewer field inventory

The `fields` tab: every native field combination the dataset holds, with its observed metadata
and links to each exact occurrence.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/data_viewer/field_inventory/
└── mod.rs  the `Fields` component: searchable, cursor-paged field combinations and occurrences
```

## Boundaries

- Depends on: the viewer's location and requests in
  `crates/frontend/workspaces/debug_benches/src/data_viewer/`, its layout components, and
  `EquipmentFieldPage` in `crates/frontend/foundation/frontend_api_dtos/src/equipment_data_viewer/`.
- Used by: `page.rs` in `crates/frontend/workspaces/debug_benches/src/data_viewer/`.
- Rules: an occurrence link names the exact resource and container, so it opens the source of
  that occurrence.
