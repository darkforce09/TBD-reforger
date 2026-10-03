**Status:** live

# Mod documentation

The deeper documents of the Arma Reforger [mod](/documentation/glossary/g_to_m.md#mod), one folder per
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) addon under `apps/mod/`: the game framework's
design and screen specifications, the export addon's pipelines and evidence, and the MCP bridge.
Developers and agents changing the mod start here after the code README of the addon.

## Contents

```text
documentation/mod/
├── script_modularisation_progress_checkpoint.md  program record: resume state of the mod script modularisation
├── tbd-emcp/                                     the Enfusion MCP bridge: call paths, bootstrap and loading rules
├── tbd-export/                                   the export addon: map export, terrain export runbook, equipment evidence
└── tbd-framework/                                the game framework: design, vanilla source coverage and the UI screen specs
```

## How it works

The folders mirror `apps/mod/`, one per addon, and inside an addon they follow the code folders
(for example `tbd-export/Scripts/WorkbenchGame/MapExport/`). Each addon's code README describes
its folders; the documents here cover what spans them.

| Addon | What it is | Documents |
|---|---|---|
| `TBD_Framework` | the game mod dedicated servers run: phases, safe start, slots and loadouts, radios, objectives and the HUD and menus | [mod design](/documentation/mod/tbd-framework/mod_design.md), [vanilla source coverage](/documentation/mod/tbd-framework/vanilla_source_coverage.md), [UI screens](/documentation/mod/tbd-framework/UI/README.md) |
| `TBD_Export` | the Workbench export tooling: elevation, roads, water, objects, building blueprints and voxel dumps, equipment, vehicles and registry items | [export addon documentation](/documentation/mod/tbd-export/README.md) |
| `TBD_EMCP` | the nineteen Net API handlers the Enfusion MCP tools call in [Workbench](/documentation/glossary/n_to_z.md#workbench) | [Enfusion MCP bridge](/documentation/mod/tbd-emcp/workbench_mcp_bridge.md) |

The procedures that span the addons live in `documentation/runbooks/`:

| Runbook | What it covers |
|---|---|
| [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) | how a mod change runs through Workbench, the gates and a slice worktree |
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
| `cargo xtask verify enfusion-comments` | the pinned mod Scripts roots meet the comment card: headers, banners, member docs and tags |

The mod script modularisation program (splitting, documenting and gating the three addons'
scripts) keeps its resume state in the
[progress checkpoint](/documentation/mod/script_modularisation_progress_checkpoint.md), beside
its ticket's spec and plan in `documentation/tickets/`.

The mod's milestone plans and agent handoffs are archived: the
[milestone plan](/documentation/archive/product_plans/mod_milestones.md), the
[first milestone announcement](/documentation/archive/product_plans/discord_milestone_1_post.md)
and the [agent continuation handoff](/documentation/archive/handoffs_and_kickoffs/mod_claude_continuation.md).

## Code

- [Mod suite](/apps/mod/) — the three addons, their dependencies and the getting-started commands.
- [Game framework](/apps/mod/tbd-framework/) — `TBD_Framework`.
- [Export addon](/apps/mod/tbd-export/) — `TBD_Export`.
- [MCP bridge addon](/apps/mod/tbd-emcp/) — `TBD_EMCP`.
- [Mod commands](/tools/commands/mod_operations/src/) — every `cargo xtask mod` command.

## Boundaries

- Depends on: the code READMEs under `apps/mod/`; the
  [README standard](/documentation/standards/readme_standard.md) and the templates in
  `documentation/standards/templates/`.
- Used by: the [mod suite README](/apps/mod/README.md), which links here.
- Rules: documents mirror the addon folder they cover; no document names a host address (the
  staging host is `TBD_SSH_HOST` in `deploy/deploy.env`); frozen evidence stays in
  `verification_evidence/` folders, indexed and never reworded.

## Related documentation

- [Mod reorganization specifications](/documentation/archive/improved_layout/mod/README.md) —
  the archived reorganization proposals for the mod suite, superseded by the
  [workspace restructure program](/documentation/restructure/README.md).
- [Glossary](/documentation/glossary/README.md) — mod, Enfusion, Workbench, EnfScript and mission
  header.
