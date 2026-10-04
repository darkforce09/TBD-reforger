# Equipment data viewer relationships

The relationships section of a resource: its outgoing references and incoming uses, each kept
with the native source that holds it.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/data_viewer/relationships/
└── mod.rs  `Relationships`: occurrences filtered by kind and view, with source and target links
```

## Boundaries

- Depends on: the viewer's location and requests in `crates/frontend/workspaces/debug_benches/src/data_viewer/`, its layout and source inspector, and
  `EquipmentRelationshipPage` in `crates/frontend/foundation/frontend_api_dtos/src/equipment_data_viewer/`.
- Used by: `ResourceDetails` in `crates/frontend/workspaces/debug_benches/src/data_viewer/resources/`.
- Rules: a selected occurrence shows its source component, property, read method and target, and
  links to both the originating property and the referenced resource.
