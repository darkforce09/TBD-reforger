**Status:** live

# Equipment and vehicle export documentation

The export addon's equipment and vehicle source exporter: the
[Workbench](/documentation/glossary/n_to_z.md#workbench) exporter that captures installed equipment,
vehicles and their gameplay dependencies as source-backed records, which xtask validates and
publishes as one immutable bundle.

## Contents

```text
documentation/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/
└── README.md  this page: how a generation is exported, validated and published
```

## How it works

The exporter's own README in the code tree describes the export, its records and the validate and
publish commands. A complete generation runs from the Workbench menu entry "Export Equipment and
Vehicles" (category `TBD`), lands under
`$profile:TBD_Export/equipment_vehicle_exports/generations/<generation_id>/`, and is checked and
sealed with:

```bash
cargo xtask mod validate-equipment-vehicle-export --input <generation_directory>
cargo xtask mod publish-equipment-vehicle-export --input <generation_directory>
```

## Code

- [Equipment and vehicle source exporter](/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/)
  — the reader, discovery, generation, serialisation, verification and the menu plugins.
- [Equipment diagnostics](/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/) and
  [vehicle diagnostics](/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/) — the diagnostic
  actions that call the same exporter and whose generations the publisher rejects.
- [Validation and publication](/tools/commands/mod_operations/src/equipment_vehicle_export/) —
  the two `cargo xtask mod` commands.
- [Export contract](/contracts/definitions/equipment-vehicle-export.schema.json) — the schema a
  generation validates against.

## Boundaries

- Depends on: the exporter and the xtask commands above.
- Used by: the exporter's README in `mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`
  and the validation README in `tools/commands/mod_operations/src/equipment_vehicle_export/`, which
  point here.
- Rules: the export contract is the one shape a generation is validated against; the publisher
  rejects diagnostic generations.

## Related documentation

- [Export addon documentation](/documentation/mod/tbd-export/README.md) — the index of every
  exporter's documents.
