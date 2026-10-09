# Mod suite

The Arma Reforger [mod](/documentation/glossary/g_to_m.md#mod) of the TBD platform, as three
[Enfusion](/documentation/glossary/a_to_f.md#enfusion) addons: the game mod that runs every TBD
session from the [mission](/documentation/glossary/g_to_m.md#mission) JSON the platform deploys, and
two [Workbench](/documentation/glossary/n_to_z.md#workbench) addons, one that exports the game data
the platform ingests and one that lets the Enfusion MCP tools drive Workbench.

## Contents

```text
mod/
├── References/     the gitignored upstream reference lanes (CRF, vanilla, PlayableSelector) and their README
├── tbd-emcp/       addon `TBD_EMCP`: the Workbench Net API handlers the MCP `wb_*` tools call
├── tbd-export/     addon `TBD_Export`: Workbench map, equipment, vehicle and registry export tooling
└── tbd-framework/  addon `TBD_Framework`: the game mod dedicated servers run
```

## How it works

Only `tbd-framework/` reaches players and servers. A dedicated server loads it as a loose addon or
from the Workshop, boots its [mission header](/documentation/glossary/g_to_m.md#mission-header), and the
framework fetches the mission deployed to that server from the website API, verifies it and runs it.
The other two addons run inside Workbench only: `tbd-export/` holds the export plugins and its own
export world, and `tbd-emcp/` holds the Net API handlers of the Enfusion MCP bridge.

At run time the framework talks to the website's [API](/documentation/glossary/a_to_f.md#api)
alone: it reads its [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) and
[event](/documentation/glossary/a_to_f.md#event) roster from the `/api/v1/game-runtime/` routes,
reports server status and match results to `/api/v1/ingest/`, and carries out the in-game
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command), such as loading a mission,
through `/api/v1/fleet-executor/`.

```text
                  addon.gproj dependencies
TBD_Framework ──▶ vanilla 58D0FB3206B6F859
TBD_EMCP      ──▶ vanilla
TBD_Export    ──▶ vanilla, TBD_EMCP

dedicated server ── loads ──▶ TBD_Framework ── HTTP ──▶ crates/api/api_server
Workbench ── opens ──▶ TBD_Export (+ TBD_EMCP) ◀── Net API ── enfusion-mcp, cargo xtask mcp
```

No addon depends on the framework, and the framework carries no Workbench scripts, so the shipping
mod stays free of editor tooling. Every command that builds, checks, boots or deploys the addons is
a `cargo xtask mod`, `mcp`, `setup` or `deploy` command in `tools/xtask/`.

| Item | Value |
|---|---|
| Addon GUIDs | `TBD_Framework` `B2C3D4E5F6A78901`, `TBD_Export` `C3D4E5F6A7B89012`, `TBD_EMCP` `D4E5F6A7B8C90123` |
| Development mission header | `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` |
| Development world | `{F652B97A6F497348}worlds/TBD_Dev_POC.ent`, a sub-scene of Eden |
| Golden mission | `msn_8f3a2c`, "Bridgehead at Levie", 18 slots (`contracts/fixtures/missions/valid/bridgehead-at-levie.json`) |
| Development server game port | 2001 (`tools/xtask/dedicated_server_profiles/tbd-dev-server.config.json`) |

## Getting started

Run these from the repository root. The gates need the Linux Arma Reforger dedicated server
installed through Steam, and the Workbench commands need Arma Reforger Tools.

```bash
cargo xtask mod compile               # compiles the framework's game scripts headless; exit 0 when clean
cargo xtask verify enfusion-comments  # the Enfusion comment card over the pinned mod Scripts roots; exit 0 when clean
cargo xtask mod world-boot            # boots the development mission header headless; exit 0 PASS
cargo xtask mod dev-bootstrap         # opens Workbench on tbd-export with the MCP bridge; exit 0 once wb_connect answers
cargo xtask mcp smoke                 # checks the live bridge with wb_connect and wb_state
```

With the API running (`cargo xtask db up`, then `cargo xtask mk rust-api` in the foreground), a
local server plays a platform mission:

```bash
cargo xtask setup server-profile                                # the profile under mod/.local-test-profile/
cargo xtask mod playtest --mission=<uuid> --admin=<identityId>  # a local dedicated server; stays in the foreground
cargo xtask mod test-game-runtime-api                           # the game-runtime routes, with TBD_MACHINE_CREDENTIAL
```

The staging fleet takes `cp deploy/deploy.env.example deploy/deploy.env`,
filled with `TBD_SSH_HOST` and the `TBD_FLEET_*` settings; on the host, the machine credentials
that `cargo xtask staging provision-fleet` writes and the join password in
`~/tbd/fleet/join-password`; then `cargo xtask deploy staging`. Each fleet instance N runs its own
dedicated server (`tbd-reforger@N`, game port `TBD_FLEET_GAME_PORT_BASE + N`) beside its own host
agent (`game_server_host_agent@N`, configured by `~/.config/game_server_host_agent/instance-N/agent.toml`);
the relay instance's agent (instance 5 on staging) reaches the API through the
acknowledgement-dropping relay. With `TBD_WORKSHOP_MOD_ID` set, clients Direct Join an instance
and download the Workshop mod; only instance 1 is listed in the server browser, and a local
`-addons` client from `cargo xtask setup client-addons` cannot Direct Join.

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

- Depends on: the vanilla Arma Reforger data addon; the website API in `crates/api/api_server/`,
  which the framework calls over HTTP; the wire shapes in `contracts/definitions/`; the pinned
  `enfusion-mcp` package in `tools/enfusion_mcp_node_package/`.
- Used by: the dedicated servers that `cargo xtask mod playtest`, `cargo xtask deploy staging` and
  the game server host agent in `crates/fleet/game_server_host_agent/` boot; the gates in
  `tools/commands/mod_operations/src/`, run by `.github/workflows/mod-gates.yml`; the Mission
  Creator in `crates/frontend/shell/frontend_application/`, which embeds the framework's alias registry; and the
  importers of the Workbench exports in `contracts/catalogs/` and `assets/terrains/`.
- Rules:
  - `mod/` holds the three addons and no Rust crate: a `Cargo.toml` placed here is a finding of
    the crate-tier law (`cargo xtask verify crate-tiers`); the website's API server, single-page
    app and offline service worker and the game server host agent are crates under `crates/`, and
    every developer tool is a crate under `tools/`.
  - The addons' scripts keep the file-length ceiling and the Enfusion comment card
    (`cargo xtask verify file-length`, `cargo xtask verify enfusion-comments`).
  - No addon depends on `tbd-framework`, and it carries no `Scripts/WorkbenchGame/`
    (`cargo xtask mod compile` exits 1 otherwise); no upstream reference code or upstream-only
    asset GUID enters it (`cargo xtask verify no-crf-leak`).
  - The MCP handlers exist once, in `tbd-emcp/`: the MCP's `wb_launch` with `gprojPath` on
    another addon copies a second set that breaks the bridge, and its `wb_cleanup` on `tbd-emcp/`
    deletes the committed set.
  - Enfusion APIs are looked up with the Enfusion MCP tools or in the vanilla sources, never
    guessed; the upstream reference lanes in `mod/References/` are read only, never
    committed, never deployed and never opened in Workbench.
  - A dedicated server takes `-config` or `-addons`, never both; `-addonsDir` combines with
    `-config` (`tools/commands/deployment/src/staging/remote/ssh_argv.rs`).
  - `resourceDatabase.rdb` in each addon is written by Workbench only.

## Related documentation

- [Mod documentation](/documentation/mod/README.md) — the index of the mod's deeper documents.
- [Mod design](/documentation/mod/tbd-framework/mod_design.md) — what the framework is for and
  its non-negotiables.
- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — how mod work runs
  through Workbench and the gates.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the MCP call path
  and its checks.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — deploying to the
  staging server and Direct Join.
- [Boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md)
  — which build a server loaded, and the mod's log lines to match.
- [Client join and mod updates](/documentation/runbooks/game_server_staging/client_join_and_mod_updates.md)
  — getting a script change to the staging server and to players.
- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — a local
  playtest.
- [Export addon documentation](/documentation/mod/tbd-export/README.md) — the map export,
  the terrain export runbook and the equipment exporter's acceptance evidence.
- [Enfusion MCP bridge](/documentation/mod/tbd-emcp/workbench_mcp_bridge.md) — the two ways to reach
  Workbench, the bootstrap and the handler loading rules.
- [Crates](/crates/README.md) — the applications and library crates of the website and the fleet.
- [Tools](/tools/README.md) — the developer tools, the ticketboard desktop viewer among them.
- [Documentation](/documentation/README.md) — the map of every deeper document.
