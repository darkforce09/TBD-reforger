# Bayonet export

Bayonets and mounted blades with their mounting, melee combat and physical properties, written under
`$profile:TBD_Export/equipment/bayonets/`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Bayonet/
├── TBD_BayonetCombatExtractor.c  melee damage and type, attack range and rate, the handling change on the rifle
├── TBD_BayonetExportPlugin.c     the bayonet plugin class; its menu attribute is commented out
├── TBD_BayonetExtractor.c        how the bayonet mounts, and its mass, volume and mesh
├── TBD_BayonetModel.c            `TBD_BayonetInfo` with mounting, combat, physical and visual parts
└── TBD_BayonetScanner.c          the addon sweep and the catalog
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_BayonetScanner.RunScan()` has both extractors fill one `TBD_BayonetInfo` per prefab and writes
`bayonets.json` and `bayonets_meta.json` through `TBD_EquipmentExportJson`. A bayonet is the one
attachment that is itself a weapon, so its combat values come from melee and damage components
rather than from attachment attributes.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: combat values are read from the melee and damage components the prefab declares, never
  derived.
