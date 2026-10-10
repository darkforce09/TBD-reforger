**Status:** live

# Mod documentation

The deeper documents of the Arma Reforger [mod](/documentation/glossary/g_to_m.md#mod), one folder per
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) addon under `mod/`: the game framework's
design and screen specifications, the export addon's pipelines, and the MCP bridge.
Developers and agents changing the mod start here after the code README of the addon.

## Contents

```text
documentation/mod/
├── tbd-emcp/       the Enfusion MCP bridge: call paths, bootstrap and loading rules
├── tbd-export/     the export addon: map export, terrain export runbook, equipment and vehicle export
└── tbd-framework/  the game framework: design, vanilla source coverage and the UI screen specs
```

## How it works

The folder sits at the mod's code path `mod/` under `documentation/`, one folder per addon,
and inside an addon the folders follow the code folders (for example
`tbd-export/Scripts/WorkbenchGame/MapExport/`). The mod's scripts have no `src/`, so the mirror
leaves out `Scripts/Game/TBD/` instead: the screen specs of
`mod/tbd-framework/Scripts/Game/TBD/UI/` sit in `tbd-framework/UI/`. Each addon's code README
describes its folders; the documents here cover what spans them. Each folder opens with a README
index, and the documents inside follow the templates in `documentation/standards/templates/`:
feature docs for a screen, a system or a cross-cutting subject, and `decisions.md` logs for the
decisions behind them.

| Addon | What it is | Documents |
|---|---|---|
| `TBD_Framework` | the game mod dedicated servers run: phases, safe start, slots and loadouts, radios, objectives and the HUD and menus | [mod design](/documentation/mod/tbd-framework/mod_design.md), [vanilla source coverage](/documentation/mod/tbd-framework/vanilla_source_coverage.md), [UI screens](/documentation/mod/tbd-framework/UI/README.md) |
| `TBD_Export` | the Workbench export tooling: elevation, roads, water, objects, building blueprints and voxel dumps, equipment, vehicles and registry items | [export addon documentation](/documentation/mod/tbd-export/README.md) |
| `TBD_EMCP` | the nineteen Net API handlers the Enfusion MCP tools call in [Workbench](/documentation/glossary/n_to_z.md#workbench) | [Enfusion MCP bridge](/documentation/mod/tbd-emcp/workbench_mcp_bridge.md) |

The framework calls the API's `/api/v1/game-runtime/` routes from inside a dedicated server; the
[API documentation](/documentation/crates/api/api_server/README.md) describes the server side.

The procedures that span the addons live in `documentation/runbooks/`:

| Runbook | What it covers |
|---|---|
| [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) | `cargo xtask mcp call`, the warm broker, exit codes and the MCP checks |
| [Spawn determinism](/documentation/runbooks/spawn_determinism.md) | the spawn and equip determinism gate, its assertions and where its log goes |
| [Game server staging](/documentation/runbooks/game_server_staging/README.md) | bootstrapping and deploying the staging server named by `TBD_SSH_HOST`, Direct Join and client setup |
| [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) | a local end-to-end playtest with two clients |

The checks that judge mod work from the command line:

| Command | Checks |
|---|---|
| `cargo xtask mod compile` | the framework's game scripts compile headless |
| `cargo xtask mod world-boot` | the development [mission header](/documentation/glossary/g_to_m.md#mission-header) boots headless |
| `cargo xtask mcp selftest` | the MCP call path, offline |
| `cargo xtask mcp smoke` | the live bridge to an open Workbench |
| `cargo xtask mod spawn-determinism` | spawn and equip give the same outcome across fresh Workbench runs |
| `cargo xtask mod spawn-verify` | a Workbench play session spawns a player into a slot |
| `cargo xtask mod remote-logs` | a dedicated server's `console.log` shows a healthy boot |
| `cargo xtask debug direct-join` | the probes behind a LAN Direct Join |
| `cargo xtask verify file-length` | the pinned mod Scripts roots hold to 500 lines per script, 1000 per test script |

## Code

- [Mod suite](/mod/) — the three addons, their dependencies and the getting-started commands.
- [Game framework](/mod/tbd-framework/) — `TBD_Framework`.
- [Export addon](/mod/tbd-export/) — `TBD_Export`.
- [MCP bridge addon](/mod/tbd-emcp/) — `TBD_EMCP`.
- [Mod commands](/tools/commands/mod_operations/src/) — every `cargo xtask mod` command.

## Boundaries

- Depends on: the code under `mod/` and its READMEs, which every document is checked against;
  the [README standard](/documentation/standards/readme_standard.md) and the templates in
  `documentation/standards/templates/`; the glossary for its terms.
- Used by: the [mod suite README](/mod/README.md) and the addons' READMEs, which link here;
  the runbooks, which link the mod's documents.
- Rules: a folder here mirrors a code folder under `mod/` and keeps its spelling; a document
  describes the committed code, and a disagreement between a document and the code is recorded
  with both places and resolved in the code's favour; no document names a host address (the
  staging host is `TBD_SSH_HOST` in `deploy/deploy.env`).

## Related documentation

- [Workspace layout](/documentation/architecture/workspace_layout.md) — the repository tree as it
  is, the mod suite included.
- [Library crate documentation](/documentation/crates/README.md) — the API server, the single-page
  app, the game server host agent and the other library crates.
- [Tool documentation](/documentation/tools/README.md) — the developer tools, the Enfusion script
  oracle and the MCP broker among them.
- [Glossary](/documentation/glossary/README.md) — mod, Enfusion, Workbench, EnfScript and mission
  header.
