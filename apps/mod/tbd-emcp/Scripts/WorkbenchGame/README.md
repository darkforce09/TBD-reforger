# Enfusion MCP Workbench scripts

The addon's WorkbenchGame script module: the scripts
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) compiles into the editor when the TBD_EMCP
addon is loaded. It holds the Enfusion MCP bridge handlers and nothing else.

## Contents

```text
apps/mod/tbd-emcp/Scripts/WorkbenchGame/
└── EnfusionMCP/  the nineteen Net API handlers the `enfusion-mcp` `wb_*` tools call
```

## How it works

Enfusion compiles `Scripts/WorkbenchGame/` only inside Workbench; a dedicated server never reads
it. When Workbench loads the addon, the handler classes in `EnfusionMCP/` register with its Net API
under their class names, and the MCP tools call them from outside the editor.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's script API.
- Used by: the `enfusion-mcp` tools and `cargo xtask mcp wbcall`, through Workbench's Net API.
- Rules: the handlers live under `EnfusionMCP/`, the folder the MCP's `wb_launch` and `wb_cleanup`
  manage in other addons, and in no second loaded addon.

## Related documentation

- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — how the MCP
  tools reach Workbench.
