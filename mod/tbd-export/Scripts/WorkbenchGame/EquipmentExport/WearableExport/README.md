# Wearable export

Clothing, personal protective equipment, load-bearing gear and backpacks from every loaded addon,
classified by the wear area they occupy and written under
`$profile:TBD_Export/equipment/wearables/`.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/WearableExport/
├── TBD_WearableExportPlugin.c  the wearable plugin class; its menu attribute is commented out
├── TBD_WearableExtractor.c     wear area, storage, physical, armour, slot and visual properties
├── TBD_WearableModel.c         the wearable data carriers
├── TBD_WearableNaming.c        display name, description, icon and readable stems
└── TBD_WearableScanner.c       the addon sweep, the category decision and the catalog writers
```

## How it works

"Export All Equipment" runs `TBD_WearableScanner.RunScan()` in its third phase. The scanner finds
the prefabs that carry wearable signals (`BaseLoadoutClothComponent`, storage and loadout areas),
resolves their inheritance, and has the extractor fill one carrier each. The category follows the
item's `LoadoutAreaType` class; a modded area subclass resolves through `typename.IsInherited()` to
the nearest mapped base area.

The catalogs are `headgear`, `face_cover`, `eyewear`, `jackets`, `pants`, `boots`, `gloves`,
`armored_vests`, `vests_and_rigs`, `backpacks` and `accessories`, plus the `wearables_master.json`
rollup, each with its `_meta.json` sidecar.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` and
  `TBD_ItemInventoryExtractor` in `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`;
  Workbench's resource search and `GameProject.GetLoadedAddons`.
- Used by: `TBD_EquipmentExportPlugin` in
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`.
- Rules: dependencies run plugin to scanner to extractor to model to the shared core; no GUID, path list or
  prefab list is written into the code.
