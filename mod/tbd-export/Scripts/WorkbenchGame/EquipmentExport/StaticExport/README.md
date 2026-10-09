# Static weapon export

Static and crew-served weapons (mortars, tripod-mounted machine guns and bare tripod mounts) with
their turret limits, sights, crew positions, mounted armament and dismantle parts, written under
`$profile:TBD_Export/equipment/statics/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/StaticExport/
├── TBD_StaticWeaponExtractor.c  each installation's turret, sight, compartment and weapon readings
├── TBD_StaticWeaponModel.c      the `TBD_StaticWeaponInfo` carrier and its parts
├── TBD_StaticWeaponNaming.c     display names, descriptions, families and factions
└── TBD_StaticWeaponScanner.c    the addon sweep, the mortar, tripod or mount decision, the catalogs
```

## How it works

"Export All Equipment" runs `TBD_StaticWeaponScanner.RunScan()` in its second phase; this domain
has no plugin of its own. The scanner finds the static emplacements of every loaded addon, the
extractor reads each one, keeping every installation apart and reading weapon capabilities with the
same readers as carried weapons, and the scanner files the result under `mortars`, `tripods` or
`mounts`. It writes `mortars.json`, `tripods.json`, `mounts.json` and the `statics_all.json`
rollup, each with its `_meta.json` sidecar.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/`, the weapon readers
  in `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Weapon/`, and
  `TBD_ItemInventoryExtractor` in `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`.
- Used by: `TBD_EquipmentExportPlugin` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`.
- Rules: every installation keeps its own record; a value the prefab does not declare stays null.
