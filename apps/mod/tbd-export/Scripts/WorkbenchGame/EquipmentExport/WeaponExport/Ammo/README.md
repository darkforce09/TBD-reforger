# WeaponExport/Ammo

Magazines, ammunition configs, and the projectiles they fire.

### Roles & Responsibilities
- `TBD_AmmoModel.c`: Data carriers for both halves of the domain. `TBD_MagazineInfo` holds capacity, caliber, magazine-well class, tracer ratio, and physical mass/volume; `TBD_ProjectileInfo` holds ballistics, warhead, tracer, and visual properties.
- `TBD_AmmoExtractor.c`: Reads one magazine or projectile prefab, walks its component graph and container ancestry, and fills a model carrier. Owns the ammunition-config indirection that links a magazine to the projectile it chambers.
- `TBD_AmmoScanner.c`: Sweeps every loaded addon for magazine and projectile prefabs, sorts them into caliber categories, and serializes each category plus the `magazines_all` / `projectiles_all` / `ammunition_all` rollups.
- `TBD_AmmoExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Ammunition & Magazines`. Writes `$profile:TBD_Export/equipment/ammunition/`.

### Call Flow & Contracts
Menu action -> `TBD_AmmoExportPlugin.Run()` -> `TBD_AmmoScanner.Scan()` discovers prefabs -> `TBD_AmmoExtractor` fills `TBD_MagazineInfo` / `TBD_ProjectileInfo` -> scanner serializes through `TBD_EquipmentExportJson` to `equipment/ammunition/magazines/<caliber>.json` and `equipment/ammunition/projectiles/<caliber>.json`, each paired with a `_meta.json` sidecar carrying the row count and UTC timestamp. Magazines reference projectiles by resource name; the pairing is never inlined.
