**Status:** live

# Vehicle field mapping

The frozen dispositions of the 224 vehicle field paths of the original export, in one file.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/field-mapping/vehicles/
└── vehicles.json  224 vehicle fields: variants, turrets, drivetrain, seats, damage, storage and systems
```

## Code

- [Equipment and vehicle source exporter](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/)
  — where each entry's `new_source` is read.

## Boundaries

- Depends on: the audited original vehicle files.
- Used by: `inventory.json` in the parent folder, which lists this group.
- Rules: each entry keeps `old_field`, `disposition`, `new_source`, `audit_findings`, `reason` and
  `observed_in` as recorded.
