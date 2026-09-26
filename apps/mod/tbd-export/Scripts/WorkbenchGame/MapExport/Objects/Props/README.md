# Props export

A placeholder for a props layer of the map export (containers, barriers, crates, street furniture
and clutter). It resolves its output path in the world open in
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) and writes nothing.

## Contents

```text
apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Props/
├── TBD_MapExportProps.c     resolves `objects/props/props.jsonl` and writes no file
└── TBD_PropsExportPlugin.c  the standalone plugin that runs it
```

## How it works

`TBD_MapExportProps.Export` checks the shared context from
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`, builds the path
`$profile:TBD_Export/<map>/objects/props/props.jsonl` (which creates the folder), logs it and
returns true. It queries no entity and opens no file. The props the map export does write come from
`TBD_MapExportObjects` in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/`, into
`props/props.jsonl` and `props/props_meta.json`.

`TBD_PropsExportPlugin` builds its own config and context and runs the exporter; `Configure` opens
the config dialog. Its `[WorkbenchPluginAttribute]` is commented out, so Workbench lists it in no
menu, and `TBD_MapExportPlugin` does not run it.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: `TBD_MapExportContext`, `TBD_MapExportConfig` and `TBD_MapExportPaths` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Core/`.
- Used by: nothing outside the folder.
- Rules: lines added stay ASCII; `cargo xtask mod compile` compiles only the framework addon, so
  these scripts compile only when Workbench loads `tbd-export`.
