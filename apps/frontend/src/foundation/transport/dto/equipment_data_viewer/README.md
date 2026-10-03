# Equipment data viewer wire types

The read-only answers of the `/api/v1/debug/equipment-data/*` routes that the equipment data
viewer decodes, one module per family of endpoints, each mirroring its schema in
`contracts/definitions/equipment-data-viewer/`.

## Contents

```text
apps/frontend/src/foundation/transport/dto/equipment_data_viewer/
├── dataset.rs            `EquipmentDatasetStatus`: the imported generation, its import state and overview
├── field_inventory.rs    `EquipmentFieldPage`: native field combinations and their occurrences
├── mod.rs                the module tree; re-exports every family within this module
├── relationships.rs      `EquipmentRelationshipPage`: a resource's forward and reverse occurrences
├── resource_cards.rs     `EquipmentResourceCardPage`: batched native container cards with their fields
├── resources.rs          `EquipmentResourcePage`: a page of the resource catalog
└── source_inspection.rs  `EquipmentSourcePage`: native values with their provenance and object links
```

## How it works

The types are plain `serde` data named by their module (`dto::equipment_data_viewer::EquipmentSourcePage`),
not re-exported flat into `dto`. `equipment_data_viewer_parity.rs` in the sibling `tests/`
folder decodes the positive and negative fixtures of `contracts/fixtures/equipment-data-viewer/`
into each page type, claiming every wire field, and expects each negative fixture to fail.

## Boundaries

- Depends on: `serde`.
- Used by: the equipment data viewer in `apps/frontend/src/workspaces/debug/data_viewer/`.
- Rules: the API's equipment data viewer handlers in
  `crates/api/api_community_content/src/handlers/equipment_data_viewer/` and the schemas
  lead, and these types follow; every wire field is claimed
  (`equipment_viewer_dataset_parity` through `equipment_viewer_resource_cards_parity` in
  `apps/frontend/src/foundation/transport/dto/tests/equipment_data_viewer_parity.rs`).
