# Inventory Item Export Hub (`Scripts/WorkbenchGame/EquipmentExport/ItemExport`)

Universal Workbench extractors that introspect loaded Reforger addons and write the inventory items catalog to `$profile:TBD_Export/equipment/items/`.

---

## Architecture Overview

Arma Reforger models inventory items as physical or functional entities carrying an `InventoryItemComponent`. These items are carried within player pockets, vest pouches, backpacks, or vehicle cargo.

This domain provides a zero-hardcoding discovery and extraction pipeline across all loaded addons. It scans every prefab, filters out already-exported hardware (firearms in `WeaponExport/` and garments/armor/backpacks in `WearableExport/`), inspects component graphs across ancestry, extracts physical, medical, radio, optical, tool, explosive, throwable, and survival properties, and classifies items into 11 dedicated sub-catalogs plus a combined master catalog.

```text
ItemExport/
├── README.md                 <-- This document
├── TBD_ItemModel.c           <-- Strongly typed data models and JSON serialization
├── TBD_ItemExtractor.c       <-- Pure container introspection (physical, medical, radio, gadget, explosive, tool, survival)
├── TBD_ItemNaming.c          <-- Display name, description, icon resolution, prefix stripping, and stem humanization
├── TBD_ItemScanner.c         <-- Addon enumeration, dynamic categorization, and catalog writers
└── TBD_ItemExportPlugin.c    <-- Workbench entry point (Plugins > TBD > "Export Inventory Items")
```

---

## Technical Contracts

1. **One-Way Dependency**:
   `ItemExportPlugin -> ItemScanner -> ItemExtractor -> ItemModel -> Core`.
   No domain imports another domain's private classes. Shared infrastructure is called directly from `EquipmentExport/Core/` (`TBD_EquipmentComponentGraph`, `TBD_EquipmentExportPaths`, `TBD_EquipmentExportJson`, `TBD_EquipmentDisplayAttributes`, `TBD_EquipmentResourceNames`).

2. **Zero Hardcoded Data**:
   No hardcoded GUIDs, file path allowlists, or mod-specific prefab lists. Prefabs are discovered dynamically via `GameProject.GetLoadedAddons` and `Workbench.SearchResources`.

3. **Dynamic Typename Inheritance**:
   Classification uses component and attribute class introspection. Mod subclasses (e.g. custom medical or radio components) resolve through `typename.IsInherited()` to their mapped base component types.

4. **Output Structure**:
   Catalogs are written under `$profile:TBD_Export/equipment/items/`:
   - `medical.json` (Tourniquets, field dressings, morphine, saline, medkits)
   - `radios.json` (Handheld transceivers, squad radios)
   - `navigation.json` (Compasses, maps, watches, GPS)
   - `binoculars.json` (Handheld binoculars, spotting scopes, rangefinders)
   - `flashlights.json` (Handheld torches, personal lights, chem lights)
   - `tools.json` (Entrenching tools, demining flags, repair kits, building tools)
   - `explosives.json` (Landmines, demolition blocks, blasting machines, detonators)
   - `throwables.json` (Fragmentation grenades, smoke grenades)
   - `weapon_parts.json` (Disassembled tripods, mortar barrels, baseplates, ballistic tables)
   - `survival.json` (Canteens, field rations, packed tents, portable jerrycans)
   - `intel_and_misc.json` (Documents, cache notes, notebooks, personal effects)
   - `items_master.json` (Rollup of all 11 categories)
   - Matching `_meta.json` sidecars per catalog.

5. **Strict File Size Limits**:
   Every source file is strictly maintained under 500 lines of code.
