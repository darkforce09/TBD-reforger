**Status:** live

# Equipment field mapping

The frozen dispositions of the equipment field paths of the original export, one file per subject;
together with the vehicle file they cover the 706 fields the parent inventory counts.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/field-mapping/equipment/
├── attachments.json           59 attachment fields
├── identity-and-indexes.json  118 identity, name and index fields
├── items.json                 103 item fields: statics, radios, storage, armour, medical and gadgets
├── magazines.json             35 magazine fields
├── optics.json                86 optic fields
├── projectiles.json           32 projectile and ammunition fields
└── weapons.json               49 weapon fields
```

## Code

- [Equipment and vehicle source exporter](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/)
  — where each entry's `new_source` is read.

## Boundaries

- Depends on: the audited original equipment files.
- Used by: `inventory.json` in the parent folder, which lists these groups.
- Rules: each entry keeps `old_field`, `disposition`, `new_source`, `audit_findings`, `reason` and
  `observed_in` as recorded.
