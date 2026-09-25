# Enfusion MCP bridge scripts

The TBD_EMCP addon's scripts: one Workbench script module and no game module, so nothing here
reaches the game or a dedicated server.

## Contents

```text
apps/mod/tbd-emcp/Scripts/
└── WorkbenchGame/  the Workbench module that holds the Enfusion MCP bridge handlers
```

## How it works

Enfusion builds one script module per folder under an addon's `Scripts/`. This addon carries only
`WorkbenchGame/`, which [Workbench](/documentation_v2/glossary.md#workbench) compiles into the
editor; with no `Game/` folder it adds nothing to the game module the framework and the dedicated
server compile.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's script API.
- Used by: the Enfusion MCP tools, through Workbench's Net API.
- Rules: the addon adds no `Game/` scripts, so loading it changes nothing a player or a dedicated
  server runs.

## Related documentation

- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — how the MCP
  tools reach Workbench.
