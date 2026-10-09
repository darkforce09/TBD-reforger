# Equipment export destination and JSON writing

Where the equipment catalogs go and how every one of them is written: the destination settings,
path creation under `$profile:`, JSON escaping and checked writes, the run's metadata, and a
[Workbench](/documentation/glossary/n_to_z.md#workbench) Net API handler that runs the standard
scanners into an isolated folder for verification.

## Contents

```text
mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/Core/ExportDestination/
├── TBD_EquipmentExportConfig.c         the destination directory and dialog settings every scanner reads
├── TBD_EquipmentExportJson.c           escaping, checked writes, sidecars, run metadata and failures
├── TBD_EquipmentExportPaths.c          path normalization, directory creation and category subfolders
├── TBD_EquipmentNativeJson.c           native scalar and ordered-array reads, failing on unsupported types
└── TBD_StandardExportVerification.c    the `TBD_StandardExportVerification` Net API handler
```

## How it works

A plugin fills a `TBD_EquipmentExportConfig` and hands it to each scanner. Every catalog path
resolves through `TBD_EquipmentExportPaths`, which creates the folders segment by segment under
`$profile:TBD_Export/equipment/`, and every file is written through `TBD_EquipmentExportJson`:

- `BeginRun` starts an export run; each catalog registers its file and the resources it holds,
  and its `_meta.json` sidecar carries the row count, the run id and the UTC timestamp.
- Numbers go through the engine's own JSON writer (`Number`), so no value is rounded; long values
  are joined from bounded slices, since the engine's `Substring` returns at most 8191 characters.
- `ExtractionError` records a failed read without making up a replacement value.
- `CompleteRun` reloads every written file as JSON and writes `export_run_meta.json` with the
  files, the errors, the loaded addons, the game build and the run status.

`TBD_StandardExportVerification` takes a `section` and a `run_id` (refused when it holds a path
separator, `..` or `:`). It answers serialization, fire-mode, door and source-property probes
directly, or runs the weapon, equipment or vehicle scanners into
`$profile:TBD_Export/standard_verification/<run_id>/` and returns that destination.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: the engine's `JsonSaveContext`, `JsonLoadContainer`, `FileIO`, `FileHandle` and
  `GameProject`; Workbench's `NetApiHandler`; for the verification handler, the scanners of
  `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/` and
  `TBD_VehicleDeepExportPlugin` in `mod/tbd-export/Scripts/WorkbenchGame/VehicleExport/`.
- Used by: every plugin and scanner of `mod/tbd-export/Scripts/WorkbenchGame/EquipmentExport/`
  and the vehicle exports, which write through `TBD_EquipmentExportJson`; the verification handler
  is reached over the Workbench Net API, and nothing in the repository calls it.
- Rules: every catalog byte passes through `TBD_EquipmentExportJson`, so escaping, numbers and the
  timestamp have one definition; a write that stores fewer characters than asked is an error.
