# Underbarrel device export

Underbarrel grenade launchers and accessories mounted below the handguard, written under
`$profile:TBD_Export/equipment/underbarrel/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Underbarrel/
├── TBD_UnderbarrelExportPlugin.c       the underbarrel plugin class; its menu attribute is commented out
├── TBD_UnderbarrelExtractor.c          how the device mounts, and its mass, volume and mesh
├── TBD_UnderbarrelLauncherExtractor.c  the secondary muzzle, its magazine wells, projectile and zeroing
├── TBD_UnderbarrelModel.c              `TBD_UnderbarrelInfo` with mounting, launcher, physical and visual parts
└── TBD_UnderbarrelScanner.c            the addon sweep and the catalog
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_UnderbarrelScanner.RunScan()` has both extractors fill one `TBD_UnderbarrelInfo` per prefab and
writes `underbarrel.json` and `underbarrel_meta.json`. The launcher carrier holds the secondary
muzzle of a device that fires, such as the M203 and GP-25; its zeroing distances arrive unordered
and repeated across the ancestry and are collected once each, sorted.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: magazine wells are foreign keys into the ammunition catalogs, never inlined rounds.
