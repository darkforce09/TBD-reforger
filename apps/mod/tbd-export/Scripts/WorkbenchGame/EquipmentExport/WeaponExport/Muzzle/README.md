# WeaponExport/Muzzle

Suppressors, flash hiders, and muzzle brakes.

### Roles & Responsibilities
- `TBD_MuzzleModel.c`: `TBD_MuzzleInfo` with mounting, modifier, physical, and visual sub-carriers. The modifier carrier records the acoustic and ballistic deltas the device applies to its host weapon.
- `TBD_MuzzleExtractor.c`: Reads one muzzle-device prefab and fills the carrier from its attachment-type declaration, sound-suppression components, and mass/volume.
- `TBD_MuzzleScanner.c`: Sweeps every loaded addon for muzzle-device prefabs and serializes the catalog.
- `TBD_MuzzleExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Muzzle Devices`. Writes `$profile:TBD_Export/equipment/muzzles/`.

### Call Flow & Contracts
Menu action -> `TBD_MuzzleExportPlugin.Run()` -> `TBD_MuzzleScanner.Scan()` -> `TBD_MuzzleExtractor` -> `TBD_MuzzleInfo` -> `equipment/muzzles/muzzles.json` plus `muzzles_meta.json`. Modifiers are exported as the raw declared values; no suppression figure is synthesized for a device that declares none.
