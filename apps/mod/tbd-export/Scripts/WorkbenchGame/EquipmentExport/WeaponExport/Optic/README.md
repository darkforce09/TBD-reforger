# WeaponExport/Optic

Optical sights and scopes: collimators, reflex and holographic sights, telescopic and variable-power scopes, launcher sights, and backup irons.

### Roles & Responsibilities
- `TBD_OpticModel.c`: `TBD_OpticInfo` with mounting, sights, physical, and visual sub-carriers. The sights carrier records magnification range, field of view, eye relief, and whether the optic is magnified at all.
- `TBD_OpticExtractor.c`: Reads one optic prefab and fills the carrier from its sights components and container ancestry.
- `TBD_OpticScanner.c`: Sweeps every loaded addon for optic prefabs and serializes the catalog.
- `TBD_OpticExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Optics & Sights`. Writes `$profile:TBD_Export/equipment/optics/`.

### Call Flow & Contracts
Menu action -> `TBD_OpticExportPlugin.Run()` -> `TBD_OpticScanner.Scan()` -> `TBD_OpticExtractor` -> `TBD_OpticInfo` -> `equipment/optics/optics.json` plus `optics_meta.json`. Omitted values serialize as JSON null, localization tokens are preserved verbatim including the leading `#`, and the mounting keys reflect only what the BaseContainer genuinely declares.
