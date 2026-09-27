# WeaponExport/Handguard

Handguards, rail systems, and foregrips.

### Roles & Responsibilities
- `TBD_HandguardModel.c`: `TBD_HandguardInfo` with mounting, rail-slot, handling, physical, and visual sub-carriers.
- `TBD_HandguardExtractor.c`: Reads the recoil and sway handling modifiers a handguard applies, and its mass and volume.
- `TBD_HandguardMountingExtractor.c`: Reads how the handguard fits a weapon and what it carries in turn — the type it presents, the types it is compatible with, the types it obstructs, and the rail slots it offers to further attachments.
- `TBD_HandguardScanner.c`: Sweeps every loaded addon for handguard and foregrip prefabs and serializes the catalog.
- `TBD_HandguardExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Handguards & Foregrips`. Writes `$profile:TBD_Export/equipment/handguards/`.

### Call Flow & Contracts
Menu action -> `TBD_HandguardExportPlugin.Run()` -> `TBD_HandguardScanner.Scan()` -> both extractors -> `TBD_HandguardInfo` -> `equipment/handguards/handguards.json` plus `handguards_meta.json`. Rail slots are exported as foreign keys (`required_attachment_type`); the platform resolves what fits them against the attachment catalogs.
