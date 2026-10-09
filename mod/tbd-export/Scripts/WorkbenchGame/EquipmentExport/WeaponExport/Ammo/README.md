# Ammunition export

Magazines, ammunition configs, the projectiles they fire and the ballistic tables those projectiles
reference, written under `$profile:TBD_Export/equipment/ammunition/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Ammo/
├── TBD_AmmoBallisticTableExporter.c  the native ballistic and wind tables the ammunition references, each once
├── TBD_AmmoCatalog.c                 the classified rows of one scan, handed from discovery to serialization
├── TBD_AmmoCatalogWriter.c           per-caliber and per-category files, the rollups and `ammunition_all`
├── TBD_AmmoExportPlugin.c            the ammunition plugin class; its menu attribute is commented out
├── TBD_AmmoMagazineExtractor.c       magazine wells, capacity, caliber, ammo type, mass and the ammo config link
├── TBD_AmmoModel.c                   `TBD_MagazineInfo` and `TBD_ProjectileInfo`
├── TBD_AmmoNaming.c                  display name, description, icon and catalog family
├── TBD_AmmoProjectileExtractor.c     ballistics, warhead, fragmentation and the meshes of one projectile
├── TBD_AmmoScanner.c                 the addon sweep that classifies each prefab and fills the catalog
└── TBD_AmmoTracerExtractor.c         tracer ratio and pattern of a magazine, tracer colour of a projectile
```

## How it works

"Export All Equipment" runs `TBD_AmmoScanner.RunScan()` in its seventh phase; the domain's own
plugin attribute is commented out.

The scanner discovers magazine and projectile prefabs; the magazine, projectile, tracer and naming
extractors fill `TBD_MagazineInfo` and `TBD_ProjectileInfo`, and the rows are filed into a
`TBD_AmmoCatalog`, magazines by caliber and projectiles by kind. `TBD_AmmoCatalogWriter` then writes
`magazines/<caliber>.json`, `projectiles/<category>.json`, a rollup for each half and the unified
`ammunition_all` catalog, each with its `_meta.json` sidecar. While scanning,
`TBD_AmmoBallisticTableExporter` collects the ballistic and wind tables the projectiles and their
effects reference and writes each table once to `ballistic_tables.json`; it simulates no trajectory
and generates no table.

This is the one domain without a single `TBD_AmmoExtractor`: a magazine and a projectile are
different objects with different carriers. A magazine references its projectile by resource name;
the pairing is never inlined.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names); `TBD_ItemInventoryExtractor` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`.
- Used by: `TBD_EquipmentExportPlugin` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`.
- Rules: the catalog holds the scan's buckets, so the writer stays stateless; a ballistic table is
  written once per run however many projectiles reference it.
