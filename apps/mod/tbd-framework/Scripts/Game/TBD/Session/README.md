# Player and admin session flows

The human-facing side of an [event](/documentation_v2/glossary/a_to_f.md#event) on the game server: picking
a mission, slotting into the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat), reading the briefing,
spectating after death, closing the round, and administering it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/
├── Admin/            the `#tbd` chat commands, the admin screen, the permission gate and audit trail
├── Briefing/         per-side briefing payloads, the ready tally and the Briefing screen
├── Lobby/            the stage watcher, the slot roster wire, the Lobby screen and pre-slot camera
├── MissionSelector/  the deployable mission list, the deployment relay and the Mission Selector
├── Players/          shared player data and the briefing's PLAYERS panel
├── PostGame/         the END banner and the DEBRIEF scoreboard
└── Spectator/        the one-life spectator camera, roster and streaming host
```

## How it works

A round passes through these folders in stage order. On `LOBBY`, `Lobby/` raises the Mission
Selector screen of `MissionSelector/`, whose top bar leads to the Lobby and Briefing tabs; on
`BRIEFING`, `Briefing/` opens the Briefing screen, whose Ready & Continue deploys the player. A
player whose life is spent enters `Spectator/`; `END` and `DEBRIEF` open the overlays of
`PostGame/`; `Admin/` works in every stage. Each folder that talks to the server follows one
pattern: models in a `TBD_<Feature>Data.c`, a server-only `TBD_<Feature>Service.c`, a modded
`SCR_PlayerController` carrying owner-scoped RPCs, a client cache in `TBD_<Feature>Client.c`, and
screens in its `UI/` folder. The screens of the pre-game tabs read `Get()` catalogs that serve mock
data from `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`.

## Authority

- Server: the services, the admin gate and audit, the ready registry, the deployable mission list
  and relay, and the spectator streaming host.
- Client: every screen, the stage watchers, the client caches and the spectator camera.
- Owner: each feature's reply RPCs, delivered to the requesting client only.
- RPCs: all on modded `SCR_PlayerController` blocks in `Briefing/`, `Lobby/`, `MissionSelector/`
  (the browser and the admin screen) and `Spectator/`; each child README lists its own.
- Replicated properties: none in this folder; the stage, the spectator policy, the end result and
  the debrief board are `TBD_FrameworkManager` properties.

## Boundaries

- Depends on: `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/` (the stage machine),
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/` (spawning, mission loading, loadouts),
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/` (the platform calls),
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/` and the shared UI library in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/`.
- Used by: `apps/mod/tbd-framework/Prefabs/Systems/TBD_GameMode.et` (the lobby, pre-slot and
  spectator components), `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` (the screen
  presets) and `TBD_FrameworkManager`.
- Rules: the server decides, and a client renders only what the server sent it; every RPC takes
  its caller from the controller it arrived on; lines added stay ASCII and `cargo xtask mod compile`
  checks that the scripts compile.

## Related documentation

- [Mod UI documents](/documentation_v2/mod/tbd-framework/UI/README.md) — the specifications of every
  screen these folders render
