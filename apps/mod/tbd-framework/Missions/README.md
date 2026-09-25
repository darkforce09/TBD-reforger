# Framework mission headers

The framework's [mission header](/documentation_v2/glossary.md#mission-header): the Enfusion
config a dedicated server or Workbench boots to load the framework's world and game mode. The
[mission](/documentation_v2/glossary.md#mission) played in it is not here; the running game loads
it from the platform.

## Contents

```text
apps/mod/tbd-framework/Missions/
├── TBD_Dev_POC.conf       the TBD Dev POC header: the Everon world with the framework game mode
└── TBD_Dev_POC.conf.meta  its resource GUID, `{69A85365FC09E2CA}`
```

## How it works

`TBD_Dev_POC.conf` is an `SCR_MissionHeader` that names the world
`{F652B97A6F497348}worlds/TBD_Dev_POC.ent`, a sub-scene of vanilla Eden (Everon) whose layer places
the framework game mode prefab. The header itself sets only what the server browser and the session
show: the name "TBD Dev POC", the author "TBD Event", the game mode label "TBD", 64 players and a
12:00 start. Its description tells an operator which log prefixes a healthy boot prints:
`[TBD][Mission] loaded id=`, then `[TBD][Slots] Slot-`, then `[TBD][Loadout][Slot]`.

```text
Missions/TBD_Dev_POC.conf ──World──▶ worlds/TBD_Dev_POC.ent ──Parent──▶ {853E92315D1D9EFE}worlds/Eden/Eden.ent
                                     worlds/TBD_Dev_POC_Layers/default.layer ──places──▶ Prefabs/Systems/TBD_GameMode.et
```

A server boots the header by its resource name, `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`,
given as the dedicated-server config's `game.scenarioId`.

## Format

- File type: an Enfusion config (`.conf`), plain text `SCR_MissionHeader { … }` with the vanilla
  fields `World`, `m_sName`, `m_sAuthor`, `m_sDescription`, `m_sGameMode`, `m_iPlayerCount` and
  `m_iStartingHours`.
- Resource GUID: the `.conf.meta` file's `Name "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"` line.
  The resource name is written into server configs, the deploy settings, the database seeds and the
  fleet host agent's tests, so it never changes.
- Naming: `TBD_<Name>.conf`, one header per world the framework boots.
- Adding a header: create it in Workbench inside this addon, which writes the `.meta` with a new
  GUID; commit the pair with the rewritten `apps/mod/tbd-framework/resourceDatabase.rdb`, and
  register its resource name as a [fleet scenario](/documentation_v2/glossary.md#fleet-scenario) for
  its terrain so the platform can boot it.

## Referenced by

- `tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json` names it as
  `game.scenarioId`; `cargo xtask mod playtest` and `cargo xtask mod world-boot` boot from that
  profile, and the world-boot verdict expects the header in the server log.
- `tools_v2/xtask/deploy/deploy.env.example` sets `TBD_SCENARIO` to it, and
  `cargo xtask deploy staging` uses it as the default (`tools_v2/xtask/src/commands/deploy/staging/config.rs`).
- `apps/website/api_v2/seeds/content_golden.sql` seeds it as the `everon` fleet scenario, which the
  platform sends to the fleet host agent in `apps/fleet_host_agent/` when it deploys a mission.
- `cargo xtask setup server-profile` names `Missions/TBD_Dev_POC.conf` in its Workbench checklist.

## Boundaries

- Depends on: the world `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent`, and through it the vanilla
  Eden world and the framework game mode prefab.
- Used by: the dedicated-server profiles and deploy settings in `tools_v2/xtask/`, the fleet
  scenario seeds in `apps/website/api_v2/seeds/`, and every server that boots the framework.
- Rules: the resource name `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` stays stable; the header
  and its `.meta` are committed together; the header carries no mission data, which comes from the
  platform at run time.

## Related documentation

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — booting the
  header on the staging server.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — booting it
  locally with `cargo xtask mod playtest`.
