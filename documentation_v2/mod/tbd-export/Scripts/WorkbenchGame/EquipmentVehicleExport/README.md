**Status:** live

# Equipment and vehicle export documentation

The acceptance evidence of the export addon's equipment and vehicle source exporter: the
[Workbench](/documentation_v2/glossary.md#workbench) exporter that captures installed equipment,
vehicles and their gameplay dependencies as source-backed records, which xtask validates and
publishes as one immutable bundle.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/
└── verification_evidence/  the frozen acceptance record: findings, field mapping, hashes, examples
```

## How it works

The exporter's own README in the code tree describes the export, its records and the validate and
publish commands; this folder holds only the evidence that accepted it. A complete generation runs
from the Workbench menu entry "Export Equipment and Vehicles" (category `TBD`), lands under
`$profile:TBD_Export/equipment_vehicle_exports/generations/<generation_id>/`, and is checked and
sealed with:

```bash
cargo xtask mod validate-equipment-vehicle-export --input <generation_directory>
cargo xtask mod publish-equipment-vehicle-export --input <generation_directory>
```

The evidence files are frozen: a later change to the exporter adds new evidence rather than
rewording these files.

## Code

- [Equipment and vehicle source exporter](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/)
  — the reader, discovery, generation, serialisation, verification and the menu plugins.
- [Equipment diagnostics](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/) and
  [vehicle diagnostics](/apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/) — the diagnostic
  actions that call the same exporter and whose generations the publisher rejects.
- [Validation and publication](/tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/) —
  the two `cargo xtask mod` commands.
- [Export contract](/contracts_v2/definitions/equipment-vehicle-export.schema.json) — the schema a
  generation validates against.

## Boundaries

- Depends on: the exporter and the xtask commands above, whose acceptance the evidence records.
- Used by: the exporter's README in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`
  and the validation README in `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/`, which
  point here.
- Rules: the evidence stays byte-for-byte as recorded; only its folder indexes change.

## Related documentation

- [Export addon documentation](/documentation_v2/mod/tbd-export/README.md) — the index of every
  exporter's documents.
