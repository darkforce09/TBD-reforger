# WeaponExport/Stock

Buttstocks.

### Roles & Responsibilities
- `TBD_StockModel.c`: `TBD_StockInfo` with mounting, slot, handling, physical, and visual sub-carriers. Structurally parallel to `TBD_HandguardModel.c`, since both describe a rail-mounted furniture piece that modifies weapon handling.
- `TBD_StockExtractor.c`: Reads one buttstock prefab and fills the carrier from its attachment slots, recoil and sway handling modifiers, and mass/volume.
- `TBD_StockScanner.c`: Sweeps every loaded addon for buttstock prefabs and serializes the catalog.
- `TBD_StockExportPlugin.c`: Workbench entry point `Plugins > TBD > Export All Buttstocks`. Writes `$profile:TBD_Export/equipment/stocks/`.

### Call Flow & Contracts
Menu action -> `TBD_StockExportPlugin.Run()` -> `TBD_StockScanner.Scan()` -> `TBD_StockExtractor` -> `TBD_StockInfo` -> `equipment/stocks/stocks.json` plus `stocks_meta.json`.
