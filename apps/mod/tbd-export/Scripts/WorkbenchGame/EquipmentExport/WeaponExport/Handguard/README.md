# WeaponExport/Handguard

Handguards, rail systems, and foregrips.

### Roles & Responsibilities
- `TBD_HandguardModel.c`: `TBD_HandguardInfo` with mounting, rail-slot, handling, physical, and visual sub-carriers.
- `TBD_HandguardExtractor.c`: Reads one handguard prefab and fills the carrier from its attachment slots, recoil and sway handling modifiers, and mass/volume.
- `TBD_HandguardScanner.c`: Sweeps every loaded addon for handguard and foregrip prefabs and serializes the catalog.
- `TBD_HandguardExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Handguards & Foregrips`. Writes `$profile:TBD_Export/equipment/handguards/`.

### Call Flow & Contracts
Menu action -> `TBD_HandguardExportPlugin.Run()` -> `TBD_HandguardScanner.Scan()` -> `TBD_HandguardExtractor` -> `TBD_HandguardInfo` -> `equipment/handguards/handguards.json` plus `handguards_meta.json`. Rail slots are exported as foreign keys (`required_attachment_type`); the platform resolves what fits them against the attachment catalogs.
