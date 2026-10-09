# Rifle export

Rifles only, read deeper than the master weapon sweep reads them, written to
`$profile:TBD_Export/equipment/rifles.json`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Rifle/
├── TBD_RifleExportPlugin.c  the rifle plugin class; its menu attribute is commented out
├── TBD_RifleExtractor.c     muzzles, magazine wells, fire modes, attachment slots, zeroing, mass and volume
├── TBD_RifleModel.c         `TBD_RifleWeaponInfo` with its muzzle, fire-mode and attachment-slot parts
├── TBD_RifleNaming.c        display name with a filename stem fallback, and the catalog family
└── TBD_RifleScanner.c       the addon sweep and the catalog
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_RifleScanner.RunScan()` has `TBD_RifleExtractor` and `TBD_RifleNaming` fill one
`TBD_RifleWeaponInfo` per rifle and writes `rifles.json` and `rifles_meta.json` at the equipment
root rather than a category subdirectory.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: magazine wells and required attachment types are foreign keys into the ammunition and
  attachment catalogs; no compatibility list is resolved here.
