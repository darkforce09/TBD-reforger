# Buttstock export

Buttstocks with their mounting, slots and handling modifiers, written under
`$profile:TBD_Export/equipment/stocks/`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WeaponExport/Stock/
├── TBD_StockExportPlugin.c       the stock plugin class; its menu attribute is commented out
├── TBD_StockExtractor.c          recoil and sway modifiers, mass and volume
├── TBD_StockModel.c              `TBD_StockInfo` with mounting, slot, handling, physical and visual parts
├── TBD_StockMountingExtractor.c  attachment type, compatible and obstructed types, the slot offered
├── TBD_StockNaming.c             display name, description and icon, searched wider than the shared reader
└── TBD_StockScanner.c            the addon sweep and the catalog
```

## How it works

Its plugin's `[WorkbenchPluginAttribute]` is commented out and "Export All Equipment" does not run
this scanner, so the domain runs only when the attribute is restored.

`TBD_StockScanner.RunScan()` has the three extractors fill one `TBD_StockInfo` per prefab and writes
`stocks.json` and `stocks_meta.json`. The model parallels the handguard domain's, since both
describe a mounted furniture piece that changes weapon handling. `TBD_StockNaming` accepts `UIInfo`
in place of `ItemDisplayName`, probes `UIInfo` on each component and makes a second pass over every
non-slot component, because buttstock prefabs often declare their strings outside the node the
shared reader looks at.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  destination, JSON writing, display attributes and resource names).
- Used by: nothing while its plugin attribute stays commented out.
- Rules: the wider name search stays in `TBD_StockNaming` and never changes the shared reader in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/PrefabNaming/`.
