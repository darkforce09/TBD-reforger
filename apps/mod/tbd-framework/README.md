# TBD Framework addon

The `TBD_Framework` Enfusion addon: the shipping game [mod](/documentation_v2/glossary.md#mod)
that runs TBD sessions on a dedicated server. It loads the
[mission](/documentation_v2/glossary.md#mission) the platform deploys to the server, stands up its
slots, loadouts, objectives, zones and radio nets, and gives players the lobby, briefing, spectator
and admin screens. It holds only TBD's own code and depends on vanilla Arma Reforger alone.

## Contents

```text
apps/mod/tbd-framework/
├── addon.gproj           the addon project: ID `TBD_Framework`, GUID, dependency, menu configs
├── Configs/              the menu presets, input contexts and key actions
├── Data/                 the alias spawn registry and the backend config template
├── Missions/             the mission header a server boots: TBD Dev POC on Everon
├── Prefabs/              the game mode with the manager components, and the player controller
├── resourceDatabase.rdb  Workbench's index of the addon's resources, written by Workbench only
├── Scripts/              the game module: platform bridge, core, game mode, session, systems, UI
├── UI/                   the widget layouts of the screens and HUD, and their textures
└── worlds/               the TBD Dev POC world on Eden and the layer that places the game mode
```

## How it works

A dedicated server boots the [mission header](/documentation_v2/glossary.md#mission-header)
`Missions/TBD_Dev_POC.conf`. Its world, `worlds/TBD_Dev_POC.ent`, is a sub-scene of vanilla Eden
whose layer places `Prefabs/Systems/TBD_GameMode.et`, and that game mode carries the framework's
manager components, so every system in `Scripts/Game/TBD/` starts from it. A vanilla scenario with
the addon loaded runs none of the framework, because each modded vanilla class checks that it is in
a framework world first.

```text
Missions/TBD_Dev_POC.conf ──▶ worlds/TBD_Dev_POC.ent ──▶ default.layer ──▶ Prefabs/Systems/TBD_GameMode.et
                                                                               │ manager components
                                                                               ▼
                               Scripts/Game/TBD/: API · Core · Gamemode · Session · Systems · UI
                                        │ HTTP, machine credential
                                        ▼
                         website API: deployment, artifact, event roster, runtime session, fleet commands
```

On the server, `Scripts/Game/TBD/Systems/Mission/` reads the deployment for this server from the
API, loads the artifact's exact bytes (from the API, or from the profile cache when the cached id
and SHA-256 match) and loads it only when its SHA-256 equals the published one. There is no default
mission: a server without a deployment stays in the LOADING stage, and one that cannot reach the
platform at boot runs the last verified cached artifact with a warning. The stage machine in
`Scripts/Game/TBD/Gamemode/` then moves the round through `LOADING`, `LOBBY`, `BRIEFING`,
`SAFE_START`, `LIVE`, `END` and `DEBRIEF`, `Scripts/Game/TBD/Systems/` stands one body per slot
and deploys players under one life, `Scripts/Game/TBD/Session/` and `UI/` give clients their
screens, and `Scripts/Game/TBD/API/` keeps a runtime session with the platform, runs its fleet
commands and reports links, lives and match results. The game module's domains, each with the detail in its
own README:

- [Platform API bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md): the runtime
  session and its heartbeats, the game-runtime and fleet-executor transport, identity linking and
  match results.
- [Framework core utilities](/apps/mod/tbd-framework/Scripts/Game/TBD/Core/README.md): the
  structured log, the alias-to-prefab resolver, server-to-player chat and SHA-256.
- [Gamemode](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/README.md): the stage machine,
  the round orchestrator, objectives and win conditions.
- [Player and admin session flows](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/README.md):
  mission selection, the lobby, the briefing, spectating, the round's end and administration.
- [Framework systems](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/README.md): the mission
  load, slot bodies and deployment, loadouts, zones, triggers, AI, audio, markers and radio.
- `Scripts/Game/TBD/UI/`: the shared menu stack, screen shell, common widgets and HUD.

The addon carries no `Scripts/WorkbenchGame/`: the Workbench export plugins live in
`apps/mod/tbd-export/` and the Enfusion MCP handlers in `apps/mod/tbd-emcp/`, so a Workbench
session on this addon alone has no MCP bridge unless `TBD_EMCP` is loaded beside it.

## Getting started

Run these from the repository root. The gates need the Linux Arma Reforger dedicated server under
`~/.local/share/Steam/steamapps/common/Arma Reforger Server/` (their install hint names Steam app
1890870).

```bash
cargo xtask mod compile           # compiles Scripts/Game headless; 0 clean, 1 code errors as file:line, 3 environment
cargo xtask mod world-boot        # boots the dev mission header headless; 0 PASS, 1 CODE, 2 usage, 3 ENVIRONMENT
cargo xtask setup server-profile  # writes the profile under apps/mod/.local-test-profile/ (or $TBD_PROFILE)
cargo xtask mod playtest --mission=<uuid> --admin=<identityId>  # a local server; stays in the foreground
```

`mod compile` exits 1 as well when `Scripts/WorkbenchGame/` exists here, and 3 when
`resourceDatabase.rdb` is stale: then open the addon in Workbench once and commit the rewritten rdb.
`mod world-boot` also fails when a component on the game mode prefab does not instantiate.
`mod playtest` needs the API running (`cargo xtask db up`, then `cargo xtask mk rust-api`): it has
the platform deploy the mission's approved artifact to the playtest server row, links the addon into
its run directory and boots the server with `-addonsDir` and `-config`; `--dry-run` prints the plan
and boots nothing, and `cargo xtask mod dev-server` with no arguments prints the usage and exits 2.

For Workbench, `cargo xtask setup workbench` links the Steam game data to `~/ArmaReforger-Base/data`
for Workbench's "Locate base game" prompt; then open `apps/mod/tbd-framework/addon.gproj`.
Workbench builds its script list when it loads a project, so a new `.c` file needs a Workbench
restart. `cargo xtask mod test-mission <golden>` stages a golden mission found under
`contracts_v2/` as the Workbench profile's cached artifact, and
`cargo xtask mod spawn-verify` plays the world through the MCP bridge and scans the log for the
slot spawn lines.

## Configuration

- `addon.gproj`: ID `TBD_Framework`, GUID `B2C3D4E5F6A78901`, title "TBD Framework", one
  dependency, the vanilla data addon `58D0FB3206B6F859`. Its `PC` and `HEADLESS` configurations set
  `MenuManagerSettings.MenuConfigs` to vanilla's `{C747AFB6B750CE9A}Configs/System/chimeraMenus.conf`
  and the addon's `{7BD1A70000000703}Configs/System/chimeraMenus.conf`; the list replaces vanilla's,
  so it keeps vanilla's entry first.
- The server profile, `$profile:` (`<profile dir>/profile/` on a dedicated server; under the
  Proton prefix `compatdata/1874910/…/ArmaReforgerWorkbench/profile/` for Workbench):

| File | Read by | Content |
|---|---|---|
| `TBD_BackendConfig.json` | `TBD_BackendConfig` | `backendUrl`, `serverToken`, `machineCredential`; copied from `Data/backend.example.json` |
| `TBD_Registry.json` | `TBD_Registry` | a copy of `Data/registry.json`, read only when the addon's copy is missing |
| `TBD_MissionArtifactCache/` | `TBD_MissionArtifactCache` | `document.json` (the last verified artifact), `identity.json` (its deployment), `received.json` (bytes awaiting verification) |
| `TBD_MissionParams.json` | `TBD_MissionParams` | `selections[]`: the launch values of the mission's parameters |
| `TBD_VariantConfig.json` | `TBD_MissionLoader` | the server's override of the mission's active variants |
| `TBD_LoadoutTest.json` | `TBD_LoadoutEquipComponent` | a web arsenal loadout export, equipped on a test character |

`cargo xtask setup server-profile` creates `<profile dir>/profile/` with mode 700, copies the two
`Data/` files, puts `SERVICE_TOKEN` (from the environment, else from the API's `.env` file) into
`serverToken` and `TBD_MACHINE_CREDENTIAL` into `machineCredential`. A credential that does not
start `tbdm_` counts as unset: no deployment is read, and the server runs the last verified cached
artifact or none. The platform loops re-read `TBD_BackendConfig.json` on every retry, at most a
minute apart, so a credential pasted in later takes effect without a restart.

## Public surface

- The addon GUID `B2C3D4E5F6A78901`, which the playtest and staging server configs load and
  `cargo xtask setup client-addons` passes as `-addons`.
- The mission header resource `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`, which the
  dedicated-server profiles, the deploy settings, the fleet scenario seeds and the fleet host agent
  name.
- `Data/registry.json`, which the Mission Creator embeds and the xtask schema and registry gates
  read.
- The profile files above, which `cargo xtask setup server-profile`, `mod playtest`,
  `mod test-mission`, `mod world-boot` and `deploy staging` write.
- The `[TBD]` log lines, whose tags `cargo xtask mod world-boot`, `cargo xtask mod remote-logs` and
  `cargo xtask mod spawn-verify` read, and the `#tbd` admin chat commands in
  `Scripts/Game/TBD/Session/Admin/`.

## Boundaries

- Depends on: the vanilla Arma Reforger data addon; over HTTP, the API's
  [game runtime](/documentation_v2/glossary.md#game-runtime) routes with the server's
  `mod_runtime` [machine credential](/documentation_v2/glossary.md#machine-credential) and its
  ingest routes with the service token; the wire shapes in `contracts_v2/definitions/`.
- Used by: the dedicated servers that `cargo xtask mod playtest`, `cargo xtask deploy staging` and
  the fleet host agent in `apps/fleet_host_agent/` boot; the gates of `cargo xtask mod` in
  `tools_v2/xtask/src/commands/mod_ops/`, which `.github/workflows/mod-gates.yml` runs; and the
  Mission Creator in `apps/website/frontend/`, through `Data/registry.json`.
- Rules: `addon.gproj` names the vanilla data addon as its only dependency, and the addon carries no
  `Scripts/WorkbenchGame/` (`cargo xtask mod compile`); no upstream reference code or upstream-only
  asset GUID enters it (`cargo xtask verify no-crf-leak`); lines added stay ASCII; every resource is
  committed with its `.meta`, and `resourceDatabase.rdb` is regenerated by Workbench, never edited.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — what the framework is for,
  its non-negotiables and the Enfusion facts it relies on.
- [Mod UI documentation](/documentation_v2/mod/tbd-framework/UI/README.md) — the specification of
  each in-game screen.
- [Capability verdicts](/documentation_v2/mod/tbd-framework/capability_verdicts.md) — the TBD
  verdict for every CRF capability and the check that enforces it.
- [TBD Framework documentation](/documentation_v2/mod/tbd-framework/README.md) — the index of the
  framework's design documents.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — booting the
  framework on the staging server, and the log lines of a healthy boot.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — a local
  playtest with `cargo xtask mod playtest`.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — how mod work runs
  through Workbench and the gates.
