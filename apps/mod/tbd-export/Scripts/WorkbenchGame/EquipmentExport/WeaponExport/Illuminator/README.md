# WeaponExport/Illuminator

Weapon lights, IR illuminators, and laser pointers.

### Roles & Responsibilities
- `TBD_IlluminatorModel.c`: `TBD_IlluminatorInfo` with mounting, capability, lens, physical, and visual sub-carriers. The lens carrier records cone angle, intensity, colour, and whether the emission is visible or infrared.
- `TBD_IlluminatorExtractor.c`: Reads how the illuminator mounts, and its mass, volume, and mesh.
- `TBD_IlluminatorEmissionExtractor.c`: Reads what the device emits — beam colour, intensity, cone angle and range, whether it is visible light or infrared, and whether it is a laser, a flashlight, or both. The engine spreads these across a light component, a laser component, and a lens configuration, so each value is searched up the container ancestry.
- `TBD_IlluminatorScanner.c`: Sweeps every loaded addon for illuminator prefabs and serializes the catalog.
- `TBD_IlluminatorExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Tactical Lights & Pointers`. Writes `$profile:TBD_Export/equipment/illuminators/`.

### Call Flow & Contracts
Menu action -> `TBD_IlluminatorExportPlugin.Run()` -> `TBD_IlluminatorScanner.Scan()` -> both extractors -> `TBD_IlluminatorInfo` -> `equipment/illuminators/illuminators.json` plus `illuminators_meta.json`. An emitter that declares no lens leaves the lens carrier null rather than substituting a default.
