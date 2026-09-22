# WeaponExport/Weapon

The master arsenal sweep: every weapon in every loaded addon, bucketed by class of weapon.

### Roles & Responsibilities
- `TBD_WeaponModel.c`: `TBD_WeaponInfo` with fire-mode, muzzle, attachment-slot, sights, ballistics, physical, and classification sub-carriers. This is the broadest carrier in the subtree and the reference shape the per-hardware domains specialize.
- `TBD_WeaponExtractor.c`: Reads what a weapon is as a physical object — mass, volume, inventory footprint, bipod and disposable flags — plus its sights zeroing and its ballistic table.
- `TBD_WeaponClassificationExtractor.c`: Decides what kind of weapon a prefab is, mapping the engine's integer weapon-type enum to the catalog's string and detecting a bipod by recursive container search.
- `TBD_WeaponMuzzleExtractor.c`: Reads each muzzle a weapon declares — magazine wells, default magazine, barrel and chamber properties — and the fire modes it supports with rate of fire and burst length.
- `TBD_WeaponMountingExtractor.c`: Reads the attachment slots a weapon offers, the type each requires, the prefab already fitted, and the types a fitted attachment obstructs.
- `TBD_WeaponNaming.c`: Reads display name, description, and icon from `UIInfo`, and derives the catalog family from the prefab path.
- `TBD_WeaponScanner.c`: Sweeps every loaded addon, sorts each weapon into `rifles`, `machine_guns`, `handguns`, `launchers`, `grenades`, `explosives`, or `underbarrel`, and serializes one catalog per category plus the `weapons_all` rollup.
- `TBD_WeaponExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Weapons (Master Arsenal)`. Writes `$profile:TBD_Export/equipment/weapons/`.

### Call Flow & Contracts
Menu action -> `TBD_WeaponExportPlugin.Run()` -> `TBD_WeaponScanner.Scan()` -> the five extractors in turn -> one `TBD_WeaponInfo` -> `equipment/weapons/<category>.json` for each of the seven categories, then `weapons_all.json`, each with a `_meta.json` sidecar. Categories are derived from the prefab's path under `Prefabs/Weapons/`, so a weapon that lives outside that tree is skipped rather than guessed at. The five extractors do not call each other except where a value genuinely spans them: the muzzle extractor cleans fire-mode localization tokens through `TBD_WeaponNaming`, and the physical extractor asks `TBD_WeaponClassificationExtractor` whether a bipod is present.
