# WeaponExport/Rifle

Rifles only, extracted deeper than the master arsenal sweep covers them.

The Workbench menu entry is currently disabled: the `[WorkbenchPluginAttribute]` block in
`TBD_RifleExportPlugin.c` is commented out, so `Export All Rifles (Deep Intrinsic)` does not appear
under `Plugins > TBD`. The scanner and its models compile and are ready to run once the attribute
is restored.

### Roles & Responsibilities
- `TBD_RifleModel.c`: `TBD_RifleWeaponInfo` plus the muzzle, fire-mode, and attachment-slot carriers it owns.
- `TBD_RifleScanner.c`: Sweeps every loaded addon for rifle prefabs and fills the carriers directly. This domain has no separate extractor; the per-prefab walk lives inside the scanner.
- `TBD_RifleExportPlugin.c`: Workbench entry point, disabled. Writes `$profile:TBD_Export/equipment/rifles.json`.

### Call Flow & Contracts
Menu action -> `TBD_RifleExportPlugin.Run()` -> `TBD_RifleScanner.Scan()` fills `TBD_RifleWeaponInfo` -> `equipment/rifles.json` plus `rifles_meta.json`, written to the equipment root rather than a category subdirectory. The export carries no pre-resolved cross-product arrays: magazine wells and required attachment types are exported as foreign keys so the platform links them against the `Ammo/` and `Attachment/` catalogs at query time.
