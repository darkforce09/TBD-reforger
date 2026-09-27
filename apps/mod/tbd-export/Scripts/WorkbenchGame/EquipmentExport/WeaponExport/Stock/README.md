# WeaponExport/Stock

Buttstocks.

### Roles & Responsibilities
- `TBD_StockModel.c`: `TBD_StockInfo` with mounting, slot, handling, physical, and visual sub-carriers. Structurally parallel to `TBD_HandguardModel.c`, since both describe a rail-mounted furniture piece that modifies weapon handling.
- `TBD_StockExtractor.c`: Reads the recoil and sway handling modifiers a buttstock applies, and its mass and volume.
- `TBD_StockMountingExtractor.c`: Reads how the buttstock fits a weapon — the type it presents, the types it is compatible with by walking the type's inheritance chain, the types it obstructs, and any slot it offers in turn.
- `TBD_StockNaming.c`: Reads display name, description, and icon. Searches wider than Core's shared reader: it accepts `UIInfo` in place of `ItemDisplayName`, probes `UIInfo` on each component, and makes a second pass over every non-slot component, because buttstock prefabs often declare their strings outside the node the shared reader looks at.
- `TBD_StockScanner.c`: Sweeps every loaded addon for buttstock prefabs and serializes the catalog.
- `TBD_StockExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Buttstocks`. Writes `$profile:TBD_Export/equipment/stocks/`.

### Call Flow & Contracts
Menu action -> `TBD_StockExportPlugin.Run()` -> `TBD_StockScanner.Scan()` -> the three extractors -> `TBD_StockInfo` -> `equipment/stocks/stocks.json` plus `stocks_meta.json`.
