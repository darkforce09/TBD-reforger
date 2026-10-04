# Equipment data viewer source rendering

How the viewer shows native values and their provenance, shared by the resource cards and the
view of a whole original document.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/data_viewer/source_inspector/
├── mod.rs             the module tree
├── provenance.rs      metadata as nested lists, every value as given
└── value_renderer.rs  value text, card summaries, paged array expansion, `DocumentInspector`
```

## Boundaries

- Depends on: the viewer's location and requests in `crates/frontend/workspaces/debug_benches/src/data_viewer/` and `EquipmentSourcePage` in
  `crates/frontend/foundation/frontend_api_dtos/src/equipment_data_viewer/`; `serde_json`.
- Used by: the cards in `crates/frontend/workspaces/debug_benches/src/data_viewer/resource_data/`, the relationships section, the
  resource header, and `page.rs` for the `document` tab.
- Rules: a number keeps its source digits, arrays expand in order one page at a time, and a
  missing value is shown as missing, never replaced; metadata nested deeper than five levels
  shows as pretty-printed JSON.
