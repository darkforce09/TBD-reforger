# WeaponExport/Underbarrel

Underbarrel grenade launchers and accessories mounted below the handguard.

### Roles & Responsibilities
- `TBD_UnderbarrelModel.c`: `TBD_UnderbarrelInfo` with mounting, launcher, physical, and visual sub-carriers. The launcher carrier holds the secondary muzzle and its magazine wells for devices that fire, such as the M203 and GP-25.
- `TBD_UnderbarrelExtractor.c`: Reads one underbarrel prefab and fills the carrier, resolving the secondary muzzle and magazine-well classes where the device is a launcher.
- `TBD_UnderbarrelScanner.c`: Sweeps every loaded addon for underbarrel prefabs and serializes the catalog.
- `TBD_UnderbarrelExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Underbarrel Devices`. Writes `$profile:TBD_Export/equipment/underbarrel/`.

### Call Flow & Contracts
Menu action -> `TBD_UnderbarrelExportPlugin.Run()` -> `TBD_UnderbarrelScanner.Scan()` -> `TBD_UnderbarrelExtractor` -> `TBD_UnderbarrelInfo` -> `equipment/underbarrel/underbarrel.json` plus `underbarrel_meta.json`. Magazine wells are foreign keys into the `Ammo/` catalogs, never inlined rounds.
