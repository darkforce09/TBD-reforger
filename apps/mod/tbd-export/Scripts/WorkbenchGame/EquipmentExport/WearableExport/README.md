# Wearable & Gear Export Hub (`Scripts/WorkbenchGame/EquipmentExport/WearableExport`)

Universal Workbench extractors that introspect loaded Reforger addons and write the clothing, personal protective equipment, load-bearing gear, and wearables catalog to `$profile:TBD_Export/equipment/wearables/`.

---

## Architecture Overview

Arma Reforger models infantry gear as modular, physically simulated entities attached to discrete **wear areas** on the character. Multiple items can be worn simultaneously on complementary layers (e.g. ballistic body armor beneath tactical load-bearing webbing).

This domain provides a zero-hardcoding discovery and extraction pipeline across all loaded addons. It scans every prefab, identifies wearable signals (`BaseLoadoutClothComponent`, storage, and equipment loadout areas), resolves inheritance chains, extracts physical/storage/armor properties, and classifies items into 10 dedicated sub-catalogs plus a combined master catalog.

```text
WearableExport/
├── README.md                   <-- This document
├── TBD_WearableModel.c         <-- Data models and JSON serialization
├── TBD_WearableExtractor.c     <-- Pure container introspection (area, storage, physical, armor, slots, visual)
├── TBD_WearableNaming.c        <-- Display name, description, icon resolution, and stem humanization
├── TBD_WearableScanner.c       <-- Addon enumeration, dynamic categorization, and catalog writers
└── TBD_WearableExportPlugin.c  <-- Workbench entry point (Plugins > TBD > "Export Wearables & Gear")
```

---

## Technical Contracts

1. **One-Way Dependency**:
   `ExportPlugin -> Scanner -> Extractor -> Model -> Core`.
   No domain imports another domain's private classes. Shared infrastructure is called directly from `EquipmentExport/Core/` (`TBD_EquipmentComponentGraph`, `TBD_EquipmentExportPaths`, `TBD_EquipmentExportJson`, `TBD_EquipmentDisplayAttributes`, `TBD_EquipmentResourceNames`).

2. **Zero Hardcoded Data**:
   No hardcoded GUIDs, file path allowlists, or mod-specific prefab lists. Prefabs are discovered dynamically via `GameProject.GetLoadedAddons` and `Workbench.SearchResources`.

3. **Dynamic Typename Inheritance**:
   Classification uses `LoadoutAreaType` class introspection. Mod subclasses (e.g. `RHS_JacketArea`) resolve through `typename.IsInherited()` to the nearest mapped base area class.

4. **Output Structure**:
   Catalogs are written under `$profile:TBD_Export/equipment/wearables/`:
   - `headgear.json` (`LoadoutHeadCoverArea`)
   - `face_cover.json` (`LoadoutCoverArea`)
   - `eyewear.json` (`LoadoutGooglesArea`)
   - `jackets.json` (`LoadoutJacketArea`)
   - `pants.json` (`LoadoutPantsArea`)
   - `boots.json` (`LoadoutBootsArea`)
   - `gloves.json` (`LoadoutHandwearSlotArea`)
   - `armored_vests.json` (`LoadoutArmoredVestSlotArea` — plate carriers & body armor)
   - `vests_and_rigs.json` (`LoadoutVestArea` — tactical vests, chest rigs, webbing)
   - `backpacks.json` (`LoadoutBackpackArea`)
   - `accessories.json` (`LoadoutWatchArea`, `LoadoutSalineBagArea`, attachable pouches)
   - `wearables_master.json` (Rollup of all categories)
   - Matching `_meta.json` sidecars per catalog.

5. **Strict File Size Limits**:
   Every source file is strictly maintained under 500 lines of code.
