# Equipment and vehicle export generation

Runs one export generation from start to `generation.json`: reader verification, the resource
queue, one record and one source snapshot per resource, the gameplay dependencies each resource
pulls in, and the environment the run saw. A generation is private staging data in the
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) profile until
`cargo xtask mod publish-equipment-vehicle-export` validates and seals it.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/Generation/
├── TBD_SourceExportEnvironment.c  game build, addon load order, settings and UTC timestamps
└── TBD_SourceExportGeneration.c   a generation's start, steps and finish
```

## How it works

```text
Start(scope, resources)
  ├─ id = Workbench.GenerateGloballyUniqueID64(); make generations/<id>/
  ├─ TBD_SourceReaderVerification.Run()          fail → finish as failed
  ├─ scope "complete": discovery census          scope "diagnostic": the given resources
  └─ sorted resources → queue (one per identity, at most 100,000)
Step()  (once per queued resource)
  ├─ reader.Capture → builder.Build (en_us locale held, then restored)
  ├─ write records/<addon>/<id prefix>/<id>.json and sources/<same path>
  ├─ index entry: resource id, name, record and source file, domains
  └─ queue every gameplay reference not yet queued
Finish()
  └─ generation.json; status "failed" when any error was recorded
```

`TBD_SourceExportGeneration.s_Active` holds the one running generation; the menu plugin and the
Net API handler both refuse to start another while it is unfinished. The directory is
`$profile:TBD_Export/equipment_vehicle_exports/generations/<generation id>/`. A record's path is
the first source addon's snake_case name, the first two characters of its resource GUID and the
GUID (`resource_<n>` when the name has none). A resource belongs to the `equipment` or `vehicle`
domain when discovery listed it there, and to `dependency` otherwise; two different resource names
with one identity are an error.

`generation.json` (`document_type` `export_generation`, schema version 2) holds the scope
(`complete` or `diagnostic`), the status, start and finish times, the environment, the resource
index, the sorted equipment and vehicle identity lists, the discovery census, every extraction
error, the reader verification result and the native type hierarchy. The environment
(`TBD_SourceExportEnvironment.Capture`) records the game build from `GetBuildVersion`, the first
line of an optional `$TBD_Export:exporter_revision.txt` stamp, the loaded addons in load order
from `GameProject.GetLoadedAddons` with their versions `unavailable` because the engine does not
expose them, and the reader settings; a missing value is `unavailable`, never guessed.

Every 25 resources and at the finish, the generation also writes
`$profile:TBD_Export/equipment_vehicle_exports/progress.json`, a transient file outside the
generation's validated file set, and prints the same line to the log.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: in `apps/mod/tbd-export/Scripts/WorkbenchGame/EquipmentVehicleExport/`, the
  discovery in `Discovery/`, the reader in `Source/`, the builder and type hierarchy in
  `Capabilities/`, `TBD_SourceExportJson` in `Serialization/` and `TBD_SourceReaderVerification` in
  `Verification/`; the engine's `Workbench`, `WidgetManager`, `GameProject`, `FileIO` and `System`.
- Used by: `TBD_EquipmentVehicleExportPlugin` and `TBD_SourceDiagnosticExport` in `Plugins/`, and
  the `start`, `step`, `status` and `diagnostic` actions of `EMCP_WB_SourceExport` in
  `Verification/`; then, as files, `cargo xtask mod validate-equipment-vehicle-export` and
  `cargo xtask mod publish-equipment-vehicle-export`
  (`tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/`).
- Rules: a generation never changes after `Finish`, and only publication makes it current; a
  diagnostic, failed or unverified generation never publishes
  (`partial_and_unverified_exports_cannot_publish` in
  `tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/tests/validation.rs`); the folder
  holds no file the index does not list (`duplicate_json_keys_and_unlisted_stale_files_fail`).

## Related documentation

- [Export validation and publication](/tools_v2/xtask/src/commands/mod_ops/equipment_vehicle_export/README.md)
  — what the two commands check and how a generation becomes current.
- [Export contract](/contracts_v2/definitions/equipment-vehicle-export.schema.json) — the
  `export_generation` document.
