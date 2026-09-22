# WeaponExport/Weapon

The master arsenal sweep: every weapon in every loaded addon, bucketed by class of weapon.

### Roles & Responsibilities
- `TBD_WeaponModel.c`: `TBD_WeaponInfo` with fire-mode, muzzle, attachment-slot, sights, ballistics, physical, and classification sub-carriers. This is the broadest carrier in the subtree and the reference shape the per-hardware domains specialize.
- `TBD_WeaponExtractor.c`: Reads any weapon prefab and walks its full component graph and container ancestry to fill the carrier.
- `TBD_WeaponScanner.c`: Sweeps every loaded addon, sorts each weapon into `rifles`, `machine_guns`, `handguns`, `launchers`, `grenades`, `explosives`, or `underbarrel`, and serializes one catalog per category plus the `weapons_all` rollup.
- `TBD_WeaponExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Weapons (Master Arsenal)`. Writes `$profile:TBD_Export/equipment/weapons/`.

### Call Flow & Contracts
Menu action -> `TBD_WeaponExportPlugin.Run()` -> `TBD_WeaponScanner.Scan()` -> `TBD_WeaponExtractor` -> `TBD_WeaponInfo` -> `equipment/weapons/<category>.json` for each of the seven categories, then `weapons_all.json`, each with a `_meta.json` sidecar. Categories are derived from the prefab's path under `Prefabs/Weapons/`, so a weapon that lives outside that tree is skipped rather than guessed at.
