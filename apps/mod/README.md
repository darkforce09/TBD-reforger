# Mod suite

The Arma Reforger [mod](/documentation_v2/glossary.md#mod) of the TBD platform, as three
[Enfusion](/documentation_v2/glossary.md#enfusion) addons: the game mod that runs every TBD
session from the [mission](/documentation_v2/glossary.md#mission) JSON the platform deploys, and
two [Workbench](/documentation_v2/glossary.md#workbench) addons, one that exports the game data
the platform ingests and one that lets the Enfusion MCP tools drive Workbench.

## Contents

```text
apps/mod/
├── .cursor/        Cursor's MCP server entry and rule for work opened at this folder
├── .gitignore      keeps reference copies, the local server profile and Workbench build output out
├── .mcp.json       the MCP server entry that starts `enfusion-mcp` for sessions opened here
├── tbd-emcp/       addon `TBD_EMCP`: the Workbench Net API handlers the MCP `wb_*` tools call
├── tbd-export/     addon `TBD_Export`: Workbench map, equipment, vehicle and registry export tooling
└── tbd-framework/  addon `TBD_Framework`: the game mod dedicated servers run
```

## How it works

Only `tbd-framework/` reaches players and servers. A dedicated server loads it as a loose addon or
from the Workshop, boots its [mission header](/documentation_v2/glossary.md#mission-header), and the
framework fetches the mission deployed to that server from the website API, verifies it and runs it.
The other two addons run inside Workbench only: `tbd-export/` holds the export plugins and its own
export world, and `tbd-emcp/` holds the Net API handlers of the Enfusion MCP bridge.

```text
                  addon.gproj dependencies
TBD_Framework ──▶ vanilla 58D0FB3206B6F859
TBD_EMCP      ──▶ vanilla
TBD_Export    ──▶ vanilla, TBD_EMCP

dedicated server ── loads ──▶ TBD_Framework ── HTTP ──▶ apps/website/api_v2
Workbench ── opens ──▶ TBD_Export (+ TBD_EMCP) ◀── Net API ── enfusion-mcp, cargo xtask mcp
```

No addon depends on the framework, and the framework carries no Workbench scripts, so the shipping
mod stays free of editor tooling. Every command that builds, checks, boots or deploys the addons is
a `cargo xtask mod`, `mcp`, `setup` or `deploy` command in `tools_v2/xtask/`.

| Item | Value |
|---|---|
| Addon GUIDs | `TBD_Framework` `B2C3D4E5F6A78901`, `TBD_Export` `C3D4E5F6A7B89012`, `TBD_EMCP` `D4E5F6A7B8C90123` |
| Development mission header | `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` |
| Development world | `{F652B97A6F497348}worlds/TBD_Dev_POC.ent`, a sub-scene of Eden |
| Golden mission | `msn_8f3a2c`, "Bridgehead at Levie", 18 slots (`contracts_v2/fixtures/missions/valid/bridgehead-at-levie.json`) |
| Development server game port | 2001 (`tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json`) |

## Getting started

Run these from the repository root. The gates need the Linux Arma Reforger dedicated server
installed through Steam, and the Workbench commands need Arma Reforger Tools.

```bash
cargo xtask mod compile         # compiles the framework's game scripts headless; exit 0 when clean
cargo xtask mod world-boot      # boots the development mission header headless; exit 0 PASS
cargo xtask mod dev-bootstrap   # opens Workbench on tbd-export with the MCP bridge; exit 0 once wb_connect answers
cargo xtask mcp smoke           # checks the live bridge with wb_connect and wb_state
```

With the API running (`cargo xtask db up`, then `cargo xtask mk rust-api` in the foreground), a
local server plays a platform mission:

```bash
cargo xtask setup server-profile                                # the profile under apps/mod/.local-test-profile/
cargo xtask mod playtest --mission=<uuid> --admin=<identityId>  # a local dedicated server; stays in the foreground
cargo xtask mod test-game-runtime-api                           # the game-runtime routes, with TBD_MACHINE_CREDENTIAL
```

The staging server takes `cp tools_v2/xtask/deploy/deploy.env.example tools_v2/xtask/deploy/deploy.env`,
filled with `TBD_SSH_HOST` and the tokens, then `cargo xtask deploy staging`. With
`TBD_SERVER_MODE=config` and `TBD_WORKSHOP_MOD_ID` set, clients Direct Join it and download the
Workshop mod; a local `-addons` client from `cargo xtask setup client-addons` cannot Direct Join.

Other mod commands:

- `cargo xtask mod spawn-verify`: play the world in Workbench and scan the log for the slot spawn
  lines.
- `cargo xtask mod remote-logs`: judge a dedicated server's `console.log` over SSH, or a local file
  with `--file`.
- `cargo xtask mod bootstrap-staging`: the one-time staging host discovery and folder setup.
- `cargo xtask mcp call <tool> '<json>'`: call one MCP tool through the warm daemon;
  `cargo xtask mcp wb-logs` scans Workbench's latest `console.log`.
- `cargo xtask setup workbench` and `cargo xtask setup mcp-game-root`: the Workbench base-game link
  and the pak farm the MCP reads.
- `cargo xtask debug direct-join`: LAN Direct Join diagnostics.
- `cargo xtask mod dev-server`: with no arguments, the usage of `mod playtest`, exit 2.

## Boundaries

- Depends on: the vanilla Arma Reforger data addon; the website API in `apps/website/api_v2/`,
  which the framework calls over HTTP; the wire shapes in `contracts_v2/definitions/`; the pinned
  `enfusion-mcp` package in `tools_v2/enfusion_mcp_node_package/`.
- Used by: the dedicated servers that `cargo xtask mod playtest`, `cargo xtask deploy staging` and
  the fleet host agent in `apps/fleet_host_agent/` boot; the gates in
  `tools_v2/xtask/src/commands/mod_ops/`, run by `.github/workflows/mod-gates.yml`; the Mission
  Creator in `apps/website/frontend/`, which embeds the framework's alias registry; and the
  importers of the Workbench exports in `contracts_v2/catalogs/` and `assets_v2/terrains/`.
- Rules:
  - No addon depends on `tbd-framework`, and it carries no `Scripts/WorkbenchGame/`
    (`cargo xtask mod compile` exits 1 otherwise); no upstream reference code or upstream-only
    asset GUID enters it (`cargo xtask verify no-crf-leak`).
  - The MCP handlers exist once, in `tbd-emcp/`: the MCP's `wb_launch` with `gprojPath` on
    another addon copies a second set that breaks the bridge, and its `wb_cleanup` on `tbd-emcp/`
    deletes the committed set.
  - Enfusion APIs are looked up with the Enfusion MCP tools or in the vanilla sources, never
    guessed; the upstream reference copies that `.gitignore` excludes are read only and never
    opened in Workbench.
  - A dedicated server takes `-config` or `-addons`, never both; `-addonsDir` combines with
    `-config` (`tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs`).
  - `resourceDatabase.rdb` in each addon is written by Workbench only.

## Related documentation

- [Mod documentation](/documentation_v2/mod/README.md) — the index of the mod's deeper documents.
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — what the framework is for and
  its non-negotiables.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how mod work runs
  through Workbench and the gates.
- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the MCP call path
  and its checks.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — deploying to the
  staging server and Direct Join.
- [Boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md)
  — which build a server loaded, and the mod's log lines to match.
- [Client join and mod updates](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md)
  — getting a script change to the staging server and to players.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — a local
  playtest.
- [Export addon documentation](/documentation_v2/mod/tbd-export/README.md) — the map export,
  the terrain export runbook and the equipment exporter's acceptance evidence.
- [Enfusion MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — the two ways to reach
  Workbench, the bootstrap and the handler loading rules.
