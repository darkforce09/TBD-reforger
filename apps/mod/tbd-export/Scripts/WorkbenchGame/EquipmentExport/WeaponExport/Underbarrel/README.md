# WeaponExport/Underbarrel

Underbarrel grenade launchers and accessories mounted below the handguard.

### Roles & Responsibilities
- `TBD_UnderbarrelModel.c`: `TBD_UnderbarrelInfo` with mounting, launcher, physical, and visual sub-carriers. The launcher carrier holds the secondary muzzle and its magazine wells for devices that fire, such as the M203 and GP-25.
- `TBD_UnderbarrelExtractor.c`: Reads how the device mounts, and its mass, volume, and mesh.
- `TBD_UnderbarrelLauncherExtractor.c`: Reads the secondary weapon a launching device adds — its own muzzle, the magazine wells it feeds from, the projectile it launches, and its zeroing distances, which arrive unordered and duplicated across the ancestry and are collected uniquely and sorted.
- `TBD_UnderbarrelScanner.c`: Sweeps every loaded addon for underbarrel prefabs and serializes the catalog.
- `TBD_UnderbarrelExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Underbarrel Devices`. Writes `$profile:TBD_Export/equipment/underbarrel/`.

### Call Flow & Contracts
Menu action -> `TBD_UnderbarrelExportPlugin.Run()` -> `TBD_UnderbarrelScanner.Scan()` -> both extractors -> `TBD_UnderbarrelInfo` -> `equipment/underbarrel/underbarrel.json` plus `underbarrel_meta.json`. Magazine wells are foreign keys into the `Ammo/` catalogs, never inlined rounds.
