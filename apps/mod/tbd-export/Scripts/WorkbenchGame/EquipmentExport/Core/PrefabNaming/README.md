# Prefab names and resource references

The readers for what a prefab shows the player (display name, description, inventory icon) and for
the resource strings it declares, turned into the form the catalogs export.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/PrefabNaming/
├── TBD_EquipmentDisplayAttributes.c  display name, description and icon, searched up the ancestor chains
└── TBD_EquipmentResourceNames.c      canonical `{GUID}path` resolution, separator normalization, catalog ids
```

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the engine's `BaseContainer` reflection and `Resource` loading.
- Used by: the scanners and extractors of
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` and the vehicle extractors in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`.
- Rules: `ResolveCanonicalResourceName` loads the resource to recover its `{GUID}path`, while
  `NormalizePathSeparators` only squares up separators and returns the reference as written; a
  domain picks one per field, and the choice decides what the catalog holds. A domain that reads
  differently (the wider name search of `TBD_StockNaming`) declares its own reader under its own
  name instead of changing these.
