**Status:** live

# Original prefab references

The 1,336 prefab resource names the original equipment and vehicle export referenced, split into
four frozen files by the first hex digit of the resource GUID; the acceptance proved that every one
of them resolves in the accepted generation.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/verification_evidence/legacy-resource-inventory/
└── guid-prefix-*.json  one sorted array of resource names per GUID range: 0-3, 4-7, 8-b and c-f
```

## Code

- [Validation and publication](/tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/) —
  the commands whose acceptance checked these references.

## Boundaries

- Depends on: the audited original export files that `legacy-input-manifest.json` in the parent
  folder hashes.
- Used by: `legacy-resource-inventory.json` in the parent folder, which lists each file with its
  count (314, 341, 351 and 330).
- Rules: the files stay as recorded and are never regenerated.
