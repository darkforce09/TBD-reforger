**Status:** live

# Map export documentation

The deeper documents of the export addon's map exporters: what each layer writes, which of them
run today, and the procedure that turns a full [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
export into a terrain's committed object and road data.

## Contents

```text
documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/
├── map_export.md               feature doc: the layers, their entry points, the pipeline and its gaps
└── terrain_export_runbook.md   runbook: full export, stage, build, verify and open the next phase
```

## Code

- [Map export scripts](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/) — the exporters;
  each layer folder's README gives its output format.
- [Runtime road export](/apps/mod/tbd-export/Scripts/Game/TBD/Export/) — the game-side road
  exporter the feature doc covers with the Workbench layers.
- [World export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/) — the `world`
  commands the runbook runs.
- [Map commands](/tools_v2/xtask/src/commands/map/) — `cargo xtask map export-terrain` and the
  blueprint and parity commands.

## Boundaries

- Depends on: the [feature doc](/documentation_v2/standards/templates/feature_doc.md) and
  [runbook](/documentation_v2/standards/templates/runbook.md) templates; the scripts under
  `apps/mod/tbd-export/Scripts/` and the xtask and developer-tools commands they cite.
- Used by: the [export addon index](/documentation_v2/mod/tbd-export/README.md) and the map export
  READMEs in `apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/` and
  `apps/mod/tbd-export/Scripts/Game/`, which link these documents.
- Rules: the layer and file detail stays in the code READMEs, and these documents link them; the
  runbook quotes the operator steps and messages as the code prints them.

## Related documentation

- [Terrain datasets](/assets_v2/terrains/README.md) — the committed data the pipeline builds.
- [Enfusion MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — the Net API that
  `cargo xtask mcp wbcall` reaches.
