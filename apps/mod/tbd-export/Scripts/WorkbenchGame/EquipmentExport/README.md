# Standard equipment exports

The [Workbench](/documentation/glossary/n_to_z.md#workbench) exporter that sweeps every loaded
addon for equipment and writes one JSON catalog per class of hardware under
`$profile:TBD_Export/equipment/`: weapons, static weapons, wearables, inventory items, weapon
attachments, optics and ammunition, plus a discovery catalog of every equipment prefab.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/
├── Core/                        what every domain shares: output, JSON, component walks, names
├── ItemExport/                  inventory items, tools and crates, in eleven category catalogs
├── StaticExport/                mortars, tripod-mounted weapons and bare mounts
├── TBD_EquipmentExportPlugin.c  "Export All Equipment": every standard scanner, then discovery
├── TBD_EquipmentScanItem.c      one discovery row: identity, root capabilities, measurements
├── TBD_EquipmentScanner.c       the discovery sweep behind `equipment_all.json`
├── WeaponExport/                weapons, attachments, optics and ammunition, one folder per domain
└── WearableExport/              clothing, protective equipment, load-bearing gear and backpacks
```

## How it works

`TBD_EquipmentExportPlugin` is the one menu entry, `Plugins > TBD > Export All Equipment`. Its
`Run` builds a `TBD_EquipmentExportConfig` (from `Core/`) with the destination directory and runs
eight phases in order, each a scanner's `RunScan()` writing its own catalogs:

```text
TBD_WeaponScanner        (WeaponExport/Weapon/)    ─▶ equipment/weapons/
TBD_StaticWeaponScanner  (StaticExport/)           ─▶ equipment/statics/
TBD_WearableScanner      (WearableExport/)         ─▶ equipment/wearables/
TBD_ItemScanner          (ItemExport/)             ─▶ equipment/items/
TBD_AttachmentScanner    (WeaponExport/Attachment/)─▶ equipment/attachments/
TBD_OpticScanner         (WeaponExport/Optic/)     ─▶ equipment/optics/
TBD_AmmoScanner          (WeaponExport/Ammo/)      ─▶ equipment/ammunition/
TBD_EquipmentScanner     (this folder)             ─▶ equipment/equipment_all.json, equipment_meta.json
```

Every domain has the same shape: a model of plain data carriers, extractors that each fill part of
one carrier from a prefab's effective configuration, a scanner that searches the addons, buckets
and serializes, and a plugin class. Only the master plugin carries a
`[WorkbenchPluginAttribute]`; the attribute of every domain plugin is commented out, so a single
domain runs through the master export or by restoring its attribute. Values are read as the prefab
declares them: an omitted value serializes as JSON null, localization tokens keep their leading
`#`, and every data file is paired with a `_meta.json` sidecar holding the row count and the UTC
generation timestamp.

`TBD_EquipmentScanner` searches the established prefab roots, skips structures, environment and
editor systems, and keeps a `TBD_EquipmentScanItem` per equipment prefab, with the native
capabilities of the root entity kept apart from components installed on child entities.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's `WorkbenchPlugin`, resource search and `NetApiHandler`; the engine's
  `BaseContainer` reflection, `JsonSaveContext` and `FileIO`; the loaded addons' prefabs; nothing
  from `tbd-framework`.
- Used by: people, through the Workbench menu; the `TBD_StandardExportVerification` Net API handler
  in `Core/`, which runs the scanners into an isolated folder; the first publication of a source
  export, which moves the unversioned `equipment/` folder into its archive
  (`tools/xtask/src/commands/mod_ops/equipment_vehicle_export/publication.rs`).
- Rules: dependencies run one way, plugin to scanner to extractor to model to `Core/`, and no
  domain uses another domain's classes; the scripts compile only when Workbench loads `tbd-export`,
  and a new script file appears after a Workbench cold restart. `cargo xtask mod compile` compiles
  the framework addon alone (`tools/xtask/src/commands/mod_ops/compile/execution.rs`).

## Related documentation

- [Equipment and vehicle source export](/apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/README.md)
  — the source-backed exporter, its gameplay dataset, and the validate and publish commands.
- [Vehicle exports](/apps/mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/README.md) — the
  vehicle catalogs written beside `equipment/`.
