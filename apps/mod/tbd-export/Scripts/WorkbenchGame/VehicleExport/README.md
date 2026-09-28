# Vehicle exports

The [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) exporter that discovers every
vehicle platform and variant in the loaded addons and writes their catalog and engineering data
under `$profile:TBD_Export/vehicles/`: mobility and drivetrain, armour hit zones, crew compartments,
turrets and mounted weapons, and fuel, storage, communications, electrical and AI configuration.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/
├── TBD_BTRExportPlugin.c                  "Export BTR-70 Deep Analysis & Info Dump": the BTR-70 family alone
├── TBD_VehicleAIConfigurationExtractor.c  each configured AI component, as declared
├── TBD_VehicleArmorExtractor.c            hit zones, health, armour thickness, damage multipliers, explosions
├── TBD_VehicleCatalogClassifier.c         platform, tactical role, seating, armament and physical figures
├── TBD_VehicleCatalogExportPlugin.c       "Export All Vehicles Catalog (Platforms & Variants)"
├── TBD_VehicleCatalogModel.c              platform and variant records of the catalog
├── TBD_VehicleCatalogScanner.c            groups the deep extractor's records into the catalog view
├── TBD_VehicleCompartmentExtractor.c      compartments, seats, crew roles, turnout, doors, entry points
├── TBD_VehicleDeepExportPlugin.c          "Export All Vehicles Deep Engineering Data (Universal Discovery)"
├── TBD_VehicleDeepExtractor.c             variant discovery; runs the domain extractors on each variant
├── TBD_VehicleDeepModel.c                 the engineering records: seats, weapons, turrets, wheels and more
├── TBD_VehicleDeepSerializer.c            platforms, variants and their domains as indented JSON
├── TBD_VehicleDrivetrainExtractor.c       engine, transmission, differentials, suspension, wheels, water
├── TBD_VehicleExportNaming.c              readable names, localization token cleanup, designations
├── TBD_VehicleExportPaths.c               path creation and chunked file writes (`TBD_VehicleExportJson`)
├── TBD_VehicleSystemsExtractor.c          fuel, storage, communications and electrical configuration
└── TBD_VehicleTurretExtractor.c           turrets, weapon stations, muzzles, ammunition, fire modes, sights
```

## How it works

```text
TBD_VehicleDeepExportPlugin ────────────────────────────────┐
TBD_BTRExportPlugin ────────────────────────────────────────┼─▶ TBD_VehicleDeepExtractor
TBD_VehicleCatalogExportPlugin ─▶ TBD_VehicleCatalogScanner ─┘        │
                                                                      ├─▶ TBD_VehicleCatalogClassifier
                                                                      └─▶ armour, compartment, drivetrain,
                                                                          systems (+ AI) and turret extractors
records ─▶ TBD_VehicleDeepSerializer ─▶ $profile:TBD_Export/vehicles/
```

`TBD_VehicleDeepExtractor` discovers the vehicle variants of every loaded addon and runs the domain
extractors on each one's effective configuration, keeping every installed instance and its
source-authored values. The three plugins use it differently:

- "Export All Vehicles Deep Engineering Data" (`ExportTo`) writes `vehicles_summary.json`,
  `vehicles_catalog.json` and `<platform id>/<platform id>_deep.json` per platform, and keeps
  `btr70/btr70_deep_dump.json` beside them.
- "Export All Vehicles Catalog" has `TBD_VehicleCatalogScanner` group the same records into
  platforms and variants and writes `vehicles_all.json`, `vehicles_summary.json` and
  `vehicles_meta.json`; the catalog, summary and platform files serialize the same vehicle objects.
- "Export BTR-70 Deep Analysis & Info Dump" filters the extractor to the BTR-70 family and writes
  `btr70/btr70_deep_dump.json` with its `_meta.json` sidecar.

Every extractor reads the engine's `BaseContainer` reflection; no vehicle table, weapon table or
default value is written into the code.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the component walk, display attributes, resource names and JSON writer in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/`, and
  `TBD_ItemInventoryExtractor` in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/ItemExport/`;
  Workbench's `WorkbenchPlugin` and resource search.
- Used by: people, through the Workbench menu; `TBD_StandardExportVerification` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/ExportDestination/`, which runs
  `TBD_VehicleDeepExportPlugin.ExportTo` and probes doors through `TBD_VehicleCompartmentExtractor`;
  the first publication of a source export, which moves the unversioned `vehicles/` folder into its
  archive (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/publication.rs`).
- Rules: a value the configuration does not declare stays null; every installed instance keeps its
  own record. The scripts compile only when Workbench loads `tbd-export`, and a new script file
  appears after a Workbench cold restart.

## Related documentation

- [Standard equipment exports](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/README.md)
  — the equipment catalogs written beside `vehicles/`, and the shared core.
- [Equipment and vehicle source export](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md)
  — the source-backed exporter and its gameplay dataset.
