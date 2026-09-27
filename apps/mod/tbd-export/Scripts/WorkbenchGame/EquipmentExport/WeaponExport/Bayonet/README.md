# WeaponExport/Bayonet

Bayonets and mounted blades.

### Roles & Responsibilities
- `TBD_BayonetModel.c`: `TBD_BayonetInfo` with mounting, melee combat, physical, and visual sub-carriers.
- `TBD_BayonetExtractor.c`: Reads how the bayonet mounts, and its mass, volume, and mesh.
- `TBD_BayonetCombatExtractor.c`: Reads what the bayonet does as a weapon — melee damage and damage type, attack range and rate, and the handling change fitting it makes to the rifle carrying it. A bayonet is the one attachment family that is itself a weapon, so these come from melee and damage components rather than from attachment attributes.
- `TBD_BayonetScanner.c`: Sweeps every loaded addon for bayonet prefabs and serializes the catalog.
- `TBD_BayonetExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Bayonets`. Writes `$profile:TBD_Export/equipment/bayonets/`.

### Call Flow & Contracts
Menu action -> `TBD_BayonetExportPlugin.Run()` -> `TBD_BayonetScanner.Scan()` -> both extractors -> `TBD_BayonetInfo` -> `equipment/bayonets/bayonets.json` plus `bayonets_meta.json`, serialized through `TBD_EquipmentExportJson`.
