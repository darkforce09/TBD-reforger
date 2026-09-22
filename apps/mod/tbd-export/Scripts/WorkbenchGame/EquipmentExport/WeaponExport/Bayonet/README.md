# WeaponExport/Bayonet

Bayonets and mounted blades.

### Roles & Responsibilities
- `TBD_BayonetModel.c`: `TBD_BayonetInfo` with mounting, melee combat, physical, and visual sub-carriers.
- `TBD_BayonetExtractor.c`: Reads one bayonet prefab and fills the carrier from its attachment-type declaration, melee damage components, and mass/volume.
- `TBD_BayonetScanner.c`: Sweeps every loaded addon for bayonet prefabs and serializes the catalog.
- `TBD_BayonetExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Bayonets`. Writes `$profile:TBD_Export/equipment/bayonets/`.

### Call Flow & Contracts
Menu action -> `TBD_BayonetExportPlugin.Run()` -> `TBD_BayonetScanner.Scan()` -> `TBD_BayonetExtractor` -> `TBD_BayonetInfo` -> `equipment/bayonets/bayonets.json` plus `bayonets_meta.json`, serialized through `TBD_EquipmentExportJson`.
