**Status:** live

# Enfusion MCP bridge documentation

The deeper document of `TBD_EMCP`, the [mod](/documentation_v2/glossary/g_to_m.md#mod) addon that carries
the nineteen Net API handlers through which the Enfusion MCP tools and `cargo xtask mcp wbcall`
drive a running [Workbench](/documentation_v2/glossary/n_to_z.md#workbench).

## Contents

```text
documentation_v2/mod/tbd-emcp/
└── workbench_mcp_bridge.md  feature doc: the two call paths, bootstrap, loading rules and gaps
```

## Code

- [TBD_EMCP addon](/apps/mod/tbd-emcp/) — the addon project, its licence and the handlers; its
  README holds the upgrade procedure.
- [Enfusion MCP handlers](/apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/) — the nineteen
  `EMCP_WB_*` handlers and the MCP tool each one serves.
- [MCP commands](/tools_v2/xtask/src/commands/mcp/) — `cargo xtask mcp call`, `wbcall`, `daemon`,
  `smoke` and `selftest`.
- [Bootstrap](/tools_v2/xtask/src/commands/mod_ops/) — `cargo xtask mod dev-bootstrap`, which
  brings the bridge up.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md); the
  code READMEs above, which hold the handler list and the command details.
- Used by: the [mod documentation index](/documentation_v2/mod/README.md) and the `TBD_EMCP` addon
  README, which link this folder.
- Rules: the broker, exit codes and environment belong to the
  [Enfusion MCP tooling runbook](/documentation_v2/runbooks/enfusion_mcp_tooling.md), and the
  handler list to the handler README; the feature doc links both instead of repeating them.

## Related documentation

- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the broker, the
  pinned server package, exit codes and the live checks.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how mod work runs
  through Workbench and the gates.
- [Map export](/documentation_v2/mod/tbd-export/Scripts/WorkbenchGame/MapExport/map_export.md) —
  the export handler reached through `cargo xtask mcp wbcall`.
