# Illuminator export

Weapon lights, infrared illuminators and laser pointers with their mounting and emission properties,
written under `$profile:TBD_Export/equipment/illuminators/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Illuminator/
├── TBD_IlluminatorEmissionExtractor.c  beam colour, intensity, cone, range, visible or infrared, laser or light
├── TBD_IlluminatorExportPlugin.c       the illuminator plugin class; its menu attribute is commented out
├── TBD_IlluminatorExtractor.c          how the illuminator mounts, and its mass, volume and mesh
├── TBD_IlluminatorModel.c              `TBD_IlluminatorInfo` with mounting, capability, lens, physical and visual parts
└── TBD_IlluminatorScanner.c            the addon sweep and the catalog
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_IlluminatorScanner.RunScan()` has both extractors fill one `TBD_IlluminatorInfo` per prefab and
writes `illuminators.json` and `illuminators_meta.json`. The engine spreads emission values across a
light component, a laser component and a lens configuration, so each value is searched up the
container ancestry.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: an emitter that declares no lens leaves the lens carrier null rather than taking a default.
