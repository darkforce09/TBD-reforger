**Status:** live

# Old-to-new field mapping

The frozen disposition of every one of the 706 field paths observed in the 160 files of the
original equipment and vehicle export: what the accepted source exporter does with each old field,
where its value now comes from, and which audit finding it answers.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/field-mapping/
├── equipment/      the equipment field paths, one file per subject
├── inventory.json  the summary: files examined, field count, disposition vocabulary and groups
└── vehicles/       the vehicle field paths
```

## How it works

`inventory.json` names the eight groups and the five dispositions every entry uses:
`replace_with_source_relationship`, `correct_source`, `remove_synthesized`, `retain` and `rename`.
Each group file is an array of entries with `old_field`, `disposition`, `new_source`,
`audit_findings`, `reason` and `observed_in`, the original files the field appeared in. The
[acceptance report](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/implementation_acceptance.md)
cites the inventory as its old-to-new field evidence. `inventory.json` records the operator's local
profile paths of the original and archived snapshots as they were captured.

## Code

- [Equipment and vehicle source exporter](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/)
  — where each `new_source` is read.
- [Export contract](/contracts_v2/definitions/equipment-vehicle-export.schema.json) — the record
  shape the new sources belong to.

## Boundaries

- Depends on: the audited original export and the accepted generation.
- Used by: the acceptance report in the parent folder.
- Rules: the files stay as recorded; the group counts add up to the inventory's 706 fields.
