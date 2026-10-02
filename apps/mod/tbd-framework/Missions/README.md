# Framework mission headers

The framework's [mission headers](/documentation/glossary/g_to_m.md#mission-header): the Enfusion
configs a dedicated server or Workbench boots to load one of the framework's worlds, Everon or
Arland, with the framework game mode. The
[mission](/documentation/glossary/g_to_m.md#mission) played in it is not here; the running game loads
it from the platform.

## Contents

```text
apps/mod/tbd-framework/Missions/
├── TBD_Dev_POC.conf              the TBD Dev POC header: the Everon world with the framework game mode
├── TBD_Dev_POC.conf.meta         its resource GUID, `{69A85365FC09E2CA}`
├── TBD_Dev_POC_Arland.conf       the TBD Dev POC Arland header: the Arland world with the same game mode
└── TBD_Dev_POC_Arland.conf.meta  its resource GUID, `{9716613D6210414A}`
```

## How it works

`TBD_Dev_POC.conf` is an `SCR_MissionHeader` that names the world
`{F652B97A6F497348}worlds/TBD_Dev_POC.ent`, a sub-scene of vanilla Eden (Everon) whose layer places
the framework game mode prefab. The header itself sets only what the server browser and the session
show: the name "TBD Dev POC", the author "TBD Event", the game mode label "TBD", 64 players and a
12:00 start. Its description tells an operator which log prefixes a healthy boot prints:
`[TBD][Mission] loaded id=`, then `[TBD][Slots] Slot-`, then `[TBD][Loadout][Slot]`.

`TBD_Dev_POC_Arland.conf` is the same header for Arland: it names
`{C664C066F1476634}worlds/TBD_Dev_POC_Arland.ent`, a sub-scene of vanilla Arland whose layer places
the same game mode prefab, and it differs from the Everon header only in that world, the name
"TBD Dev POC Arland" and the world its description names.

```text
Missions/TBD_Dev_POC.conf ──World──▶ worlds/TBD_Dev_POC.ent ──Parent──▶ {853E92315D1D9EFE}worlds/Eden/Eden.ent
                                     worlds/TBD_Dev_POC_Layers/default.layer ──places──▶ Prefabs/Systems/TBD_GameMode.et
Missions/TBD_Dev_POC_Arland.conf ──World──▶ worlds/TBD_Dev_POC_Arland.ent ──Parent──▶ {A9806AF617972E97}worlds/Arland/Arland.ent
                                            worlds/TBD_Dev_POC_Arland_Layers/default.layer ──places──▶ Prefabs/Systems/TBD_GameMode.et
```

A server boots a header by its resource name, given as the dedicated-server config's
`game.scenarioId`: `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` for Everon,
`{9716613D6210414A}Missions/TBD_Dev_POC_Arland.conf` for Arland. Switching a server between the two
changes its terrain, so the fleet host agent restarts the server process for it.

## Format

- File type: an Enfusion config (`.conf`), plain text `SCR_MissionHeader { … }` with the vanilla
  fields `World`, `m_sName`, `m_sAuthor`, `m_sDescription`, `m_sGameMode`, `m_iPlayerCount` and
  `m_iStartingHours`.
- Resource GUID: each `.conf.meta` file's `Name` line,
  `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` and
  `{9716613D6210414A}Missions/TBD_Dev_POC_Arland.conf`. A resource name is written into server
  configs, the deploy settings, the database seeds and the fleet host agent's tests, so it never
  changes.
- Naming: `TBD_<Name>.conf`, one header per world the framework boots.
- Adding a header: create it in Workbench inside this addon, which writes the `.meta` with a new
  GUID; commit the pair with the rewritten `apps/mod/tbd-framework/resourceDatabase.rdb`, and
  register its resource name as a [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario) for
  its terrain so the platform can boot it.

## Referenced by

- `tools/xtask/dedicated_server_profiles/tbd-dev-server.config.json` names it as
  `game.scenarioId`; `cargo xtask mod playtest` and `cargo xtask mod world-boot` boot from that
  profile, and the world-boot verdict expects the header in the server log.
- `deploy/deploy.env.example` sets `TBD_SCENARIO` to it, and
  `cargo xtask deploy staging` uses it as the default (`tools/xtask/src/commands/deploy/staging/config.rs`).
- `apps/api/seeds/content_golden.sql` seeds it as the `everon` fleet scenario, which the
  platform sends to the fleet host agent in `apps/fleet_host_agent/` when it deploys a mission.
- `cargo xtask setup server-profile` names `Missions/TBD_Dev_POC.conf` in its Workbench checklist.
- `TBD_Dev_POC_Arland.conf` is named by no committed file: a server boots it when the fleet
  scenario registered for `arland` names `{9716613D6210414A}Missions/TBD_Dev_POC_Arland.conf`.

## Boundaries

- Depends on: the worlds `apps/mod/tbd-framework/worlds/TBD_Dev_POC.ent` and
  `apps/mod/tbd-framework/worlds/TBD_Dev_POC_Arland.ent`, and through them the vanilla Eden and
  Arland worlds and the framework game mode prefab.
- Used by: the dedicated-server profiles and deploy settings in `tools/xtask/`, the fleet
  scenario seeds in `apps/api/seeds/`, and every server that boots the framework.
- Rules: the resource names `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` and
  `{9716613D6210414A}Missions/TBD_Dev_POC_Arland.conf` stay stable; a header and its `.meta` are
  committed together; the header carries no mission data, which comes from the
  platform at run time.

## Related documentation

- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — booting the
  header on the staging server.
- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — booting it
  locally with `cargo xtask mod playtest`.
