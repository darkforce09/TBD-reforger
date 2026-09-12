# TBD EMCP

Workbench Net API handlers for the [enfusion-mcp](https://www.npmjs.com/package/enfusion-mcp) bridge,
packaged as a standalone addon so a Workbench session gets the `wb_*` MCP tools without any shipping
addon carrying dev-only code.

Mod GUID: `D4E5F6A7B8C90123` · ID `TBD_EMCP` · Vanilla dependency: `58D0FB3206B6F859`

## What it is

- `Scripts/WorkbenchGame/EnfusionMCP/EMCP_WB_*.c` — the 19 handlers shipped in `enfusion-mcp@0.6.1`
  (`scripts/mod/node_modules/enfusion-mcp/mod/Scripts/WorkbenchGame/EnfusionMCP/`, MIT — see `LICENSE`),
  plus one local patch: `EMCP_WB_ScriptEditor.c` adds the `getAllText` action (dumps the whole open
  file). The package copy has no such action.
- No `Scripts/Game`, no prefabs, no configs. Nothing here ships: `cargo xtask deploy staging` excludes
  `apps/mod/tbd-emcp/` and the dedicated server never lists it.

## How it is loaded

- `apps/mod/tbd-export/addon.gproj` declares TBD_EMCP as a dependency, so opening tbd-export in
  Workbench loads tbd-framework + TBD_EMCP + tbd-export — the full dev session with the bridge alive.
  `cargo xtask mod dev-bootstrap` launches exactly that.
- To use the bridge with tbd-framework alone, load TBD_EMCP beside it in Workbench. tbd-framework does
  not depend on it by design: the shipping mod stays free of Workbench tooling.

## Rules

- **Committed, not injected.** Never call the MCP's `wb_launch` with `gprojPath` on tbd-framework or
  tbd-export: it copies a second set of these handlers into that addon's
  `Scripts/WorkbenchGame/EnfusionMCP/` and Workbench fails the WorkbenchGame module on
  "Multiple declaration" (bridge dead). Launch via `cargo xtask mod dev-bootstrap` or
  `steam -applaunch 1874910 -gproj <path>`.
- **Never call `wb_cleanup` with this directory** — it `rm -rf`s `Scripts/WorkbenchGame/EnfusionMCP/`.
- Handlers live in exactly one loaded addon. `cargo xtask mod compile` fails if the same relative path
  exists in two of the three addons.

## Upgrading enfusion-mcp

1. Bump `scripts/mod/package.json`, run `npm ci` in `scripts/mod/`.
2. `diff -r scripts/mod/node_modules/enfusion-mcp/mod/Scripts/WorkbenchGame/EnfusionMCP apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP`
   — expect only the `getAllText` patch.
3. Copy the new handlers over, re-apply the patch, cold-restart Workbench on tbd-export (the script
   list is built at load), run `cargo xtask mcp smoke`.
