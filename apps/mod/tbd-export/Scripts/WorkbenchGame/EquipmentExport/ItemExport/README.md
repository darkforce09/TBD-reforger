# Inventory item export

The inventory items of every loaded addon (medical supplies, radios, navigation aids, binoculars,
lights, tools, explosives, throwables, weapon parts, survival gear and intel), classified into
eleven catalogs under `$profile:TBD_Export/equipment/items/`.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/
├── TBD_ItemExportPlugin.c        the item plugin class; its menu attribute is commented out
├── TBD_ItemExtractor.c           physical, medical, radio, gadget, explosive, tool and survival properties
├── TBD_ItemInventoryExtractor.c  inventory attributes, body mass and individual storage compartments
├── TBD_ItemModel.c               the item data carriers
├── TBD_ItemNaming.c              display name, description, icon, prefix stripping and readable stems
└── TBD_ItemScanner.c             the addon sweep, the category decision and the catalog writers
```

## How it works

"Export All Equipment" runs `TBD_ItemScanner.RunScan()` in its fourth phase. The scanner searches
every loaded addon for prefabs carrying an `InventoryItemComponent`, skips the hardware other
domains export (firearms go to the weapon exports; garments, armour and backpacks to the wearables),
walks each prefab's components across its ancestry, and has the extractors fill one item carrier.
Classification reads component and attribute classes, and a modded subclass resolves through
`typename.IsInherited()` to the base type it extends, so no GUID, path list or prefab list is
written into the code.

Each item lands in one of `medical`, `radios`, `navigation`, `binoculars`, `flashlights`, `tools`,
`explosives`, `throwables`, `weapon_parts`, `survival` or `intel_and_misc`; the scanner writes one
catalog per category and the `items_master.json` rollup, each with its `_meta.json` sidecar.
`TBD_ItemInventoryExtractor` is shared: the wearable, weapon, static weapon, attachment, optic and
ammunition extractors and the discovery sweep read inventory attributes and storage through it.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/` (component walk,
  paths, JSON writing, display attributes, resource names); Workbench's resource search and
  `GameProject.GetLoadedAddons`.
- Used by: `TBD_EquipmentExportPlugin` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`; `TBD_ItemInventoryExtractor` also by
  the other domains' extractors, the discovery sweep, the display reader `TBD_EquipmentDisplayAttributes`
  and the vehicle extractors.
- Rules: dependencies run plugin to scanner to extractor to model to the shared core; an item belongs to
  exactly one category.
