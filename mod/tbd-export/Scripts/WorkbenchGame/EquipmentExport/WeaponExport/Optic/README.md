# Optic export

Optical sights and scopes (collimators, reflex and holographic sights, fixed and variable scopes,
launcher sights and backup irons) written under `$profile:TBD_Export/equipment/optics/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Optic/
├── TBD_OpticExportPlugin.c     the optic plugin class; its menu attribute is commented out
├── TBD_OpticExtractor.c        how the optic mounts, and its mass, volume and mesh
├── TBD_OpticModel.c            `TBD_OpticInfo` with mounting, sights, physical and visual parts
├── TBD_OpticScanner.c          the addon sweep and the catalog
└── TBD_OpticSightsExtractor.c  magnification, field of view, eye relief, zeroing, reticle and ranging
```

## How it works

"Export All Equipment" runs `TBD_OpticScanner.RunScan()` in its sixth phase; the domain's own plugin
attribute is commented out.

The scanner has both extractors fill one `TBD_OpticInfo` per prefab and writes `optics.json` and
`optics_meta.json`. The sights extractor fills the whole sights carrier in one pass up the container
ancestry, because a variant prefab declares only what it changes.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names); `TBD_ItemInventoryExtractor` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`;
  `TBD_AttachmentMountingExtractor` from
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Attachment/`.
- Used by: `TBD_EquipmentExportPlugin` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`;
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Weapon/` reads a weapon's
  own sights through `TBD_OpticSightsExtractor`.
- Rules: the mounting keys reflect only what the container declares; mounting is read through
  `TBD_AttachmentMountingExtractor` from
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Attachment/`.
