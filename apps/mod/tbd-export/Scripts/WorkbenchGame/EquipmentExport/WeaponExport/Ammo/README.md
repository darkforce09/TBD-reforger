# WeaponExport/Ammo

Magazines, ammunition configs, and the projectiles they fire.

### Roles & Responsibilities
- `TBD_AmmoModel.c`: Data carriers for both halves of the domain. `TBD_MagazineInfo` holds capacity, caliber, magazine-well class, tracer ratio, and physical mass/volume; `TBD_ProjectileInfo` holds ballistics, warhead, tracer, and visual properties.
- `TBD_AmmoMagazineExtractor.c`: Reads one magazine prefab — the wells it fits, its capacity, its caliber and ammo type, its empty and loaded mass — and files it under a caliber category. Owns the ammunition-config indirection that links a magazine to the projectile it chambers.
- `TBD_AmmoProjectileExtractor.c`: Reads one projectile prefab — muzzle velocity, mass, drag and ballistic table; warhead yield, penetration, fragmentation and effect prefab; the meshes it renders as round and cartridge — and files it under a caliber category.
- `TBD_AmmoTracerExtractor.c`: Reads tracer behaviour for both halves: the tracer ratio and pattern a magazine is loaded with, and whether a projectile is itself a tracer and what colour it burns.
- `TBD_AmmoNaming.c`: Reads display name, description, and icon from `UIInfo` and `ItemDisplayName`, and derives the catalog family from the prefab path.
- `TBD_AmmoCatalog.c`: `TBD_AmmoCatalog` holds the classified rows of one scan — magazines bucketed by caliber, projectiles by kind, and a rollup of each half. It is the hand-off between discovery and serialization, and holding the buckets here is what lets the writer stay stateless.
- `TBD_AmmoScanner.c`: Sweeps every loaded addon for magazine and projectile prefabs, classifies each one, and fills a `TBD_AmmoCatalog`.
- `TBD_AmmoCatalogWriter.c`: Serializes a catalog to disk — one file per magazine caliber, one per projectile category, a rollup for each half, and the unified `ammunition_all` catalog, each with its `_meta.json` sidecar.
- `TBD_AmmoExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Ammunition & Magazines`. Writes `$profile:TBD_Export/equipment/ammunition/`.

### Call Flow & Contracts
Menu action -> `TBD_AmmoExportPlugin.Run()` -> `TBD_AmmoScanner.RunScan()` discovers prefabs -> the magazine, projectile, tracer and naming extractors fill `TBD_MagazineInfo` / `TBD_ProjectileInfo` -> rows are filed into `TBD_AmmoCatalog` -> `TBD_AmmoCatalogWriter` serializes through `TBD_EquipmentExportJson` to `equipment/ammunition/magazines/<caliber>.json` and `equipment/ammunition/projectiles/<caliber>.json`, each paired with a `_meta.json` sidecar carrying the row count and UTC timestamp. Magazines reference projectiles by resource name; the pairing is never inlined.

This is the one domain with no single `TBD_AmmoExtractor`. A magazine and a projectile are different objects with different carriers, so the domain splits along that line rather than routing both through one class that would share nothing but a name.
