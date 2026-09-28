# Equipment data viewer field inventory

The `fields` tab: every native field combination the dataset holds, with its observed metadata
and links to each exact occurrence.

## Contents

```text
apps/website/frontend/src/v2/apps/debug/data_viewer/field_inventory/
└── mod.rs  the `Fields` component: searchable, cursor-paged field combinations and occurrences
```

## Boundaries

- Depends on: the viewer's location and requests in
  `apps/website/frontend/src/v2/apps/debug/data_viewer/`, its layout components, and
  `EquipmentFieldPage` in `apps/website/frontend/src/v2/core/api/dto/equipment_data_viewer/`.
- Used by: `page.rs` in `apps/website/frontend/src/v2/apps/debug/data_viewer/`.
- Rules: an occurrence link names the exact resource and container, so it opens the source of
  that occurrence.
