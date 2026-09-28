# Weapon export

The master weapon sweep: every weapon in every loaded addon, sorted into nine categories and written
under `$profile:TBD_Export/equipment/weapons/`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Weapon/
├── TBD_WeaponClassificationExtractor.c  the weapon type string and the bipod search
├── TBD_WeaponExportPlugin.c             the weapon plugin class; its menu attribute is commented out
├── TBD_WeaponExtractor.c                mass, volume, inventory footprint, flags, sights zeroing and ballistic table
├── TBD_WeaponModel.c                    `TBD_WeaponInfo`, the broadest carrier, and its parts
├── TBD_WeaponMountingExtractor.c        attachment slots, required types, fitted prefabs, obstructed types
├── TBD_WeaponMuzzleExtractor.c          muzzles, magazine wells, barrel and chamber, fire modes
├── TBD_WeaponNaming.c                   display name, description, icon and catalog family
└── TBD_WeaponScanner.c                  the addon sweep, the nine categories and the catalogs
```

## How it works

"Export All Equipment" runs `TBD_WeaponScanner.RunScan()` in its first phase; the domain's own
plugin attribute is commented out.

The scanner sorts each weapon into `rifles`, `machine_guns`, `handguns`, `launchers`, `flares`,
`heavy_weapons`, `grenades`, `explosives` or `underbarrel` by the prefab's folder under
`Prefabs/Weapons/`, so a weapon outside that tree is skipped rather than guessed at. The five
extractors fill one `TBD_WeaponInfo`, and the scanner writes `<category>.json` for each category and
`weapons_all.json`, each with its `_meta.json` sidecar.

The extractors call each other only where a value spans them: the muzzle extractor cleans fire-mode
localization tokens through `TBD_WeaponNaming`, and the physical extractor asks
`TBD_WeaponClassificationExtractor` whether a bipod is present. Mounting and sights come through
`TBD_AttachmentMountingExtractor` and `TBD_OpticSightsExtractor`.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names); `TBD_ItemInventoryExtractor` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`;
  `TBD_AttachmentMountingExtractor` and `TBD_AttachmentSlotInfo` from
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Attachment/`;
  `TBD_OpticSightsExtractor` from
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Optic/`.
- Used by: `TBD_EquipmentExportPlugin` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`; the static weapon extractor in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/StaticExport/`, through the muzzle and
  mounting extractors; the verification handler in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/ExportDestination/`, which runs
  the scanner and probes fire modes.
- Rules: a weapon's category comes from its prefab folder alone; a weapon outside `Prefabs/Weapons/`
  is skipped.
