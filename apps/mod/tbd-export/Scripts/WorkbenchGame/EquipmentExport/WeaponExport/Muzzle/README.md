# Muzzle device export

Suppressors, flash hiders and muzzle brakes with the acoustic and ballistic modifiers they apply,
written under `$profile:TBD_Export/equipment/muzzles/`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Muzzle/
├── TBD_MuzzleExportPlugin.c  the muzzle plugin class; its menu attribute is commented out
├── TBD_MuzzleExtractor.c     attachment type, suppression components, mass and volume of one device
├── TBD_MuzzleModel.c         `TBD_MuzzleInfo` with mounting, modifier, physical and visual parts
└── TBD_MuzzleScanner.c       the addon sweep and the catalog
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_MuzzleScanner.RunScan()` has `TBD_MuzzleExtractor` fill one `TBD_MuzzleInfo` per prefab and
writes `muzzles.json` and `muzzles_meta.json`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: modifiers are exported as the declared values; no suppression figure is made up for a
  device that declares none.
