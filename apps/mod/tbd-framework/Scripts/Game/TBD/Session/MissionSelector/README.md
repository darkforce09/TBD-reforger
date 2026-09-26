# Mission selection and in-game deployment

How server administrators pick what runs next from inside the game: the list of missions the
platform lets this server deploy, the admin browser keys that step through it, and the deployment
request relayed to the platform as a [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment).
The folder also holds the Mission Selector screen's models and the admin screen's RPC transport.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/
├── TBD_DeployableMissionList.c   the missions the platform lets this server deploy, numbered from 1
├── TBD_MissionBrowser.c          modded `SCR_PlayerController`: admin keys, browser and admin RPCs
├── TBD_MissionDeploymentRelay.c  relays an admin's pick to the platform as a deployment request
├── TBD_MissionSelectorData.c     selector models, `TBD_MissionCatalog` and `TBD_SessionSelection`
└── UI/                           the Mission Selector screen: terrains, missions and the inspector
```

## How it works

```text
LOBBY stage, or "#tbd refresh"
  -> TBD_DeployableMissionList.Refresh: GET /api/v1/game-runtime/missions
     "n) Title [terrain]", marked "(running)" or "(running another version)"
F6 (TBD_MissionCycle) -> TBD_RpcAsk_MissionList --RPC--> admin check -> list payload
                      <--RPC-- TBD_RpcDo_ReceiveMissionList: lines printed to the client log;
                               further presses step through them
F7 (TBD_MissionLoad)  -> TBD_RpcAsk_SelectMission(n) --RPC--> admin check
"#tbd mission <n>"    -> TBD_MissionDeploymentRelay.RequestByNumber(admin, n)
     POST /api/v1/game-runtime/deployments {mission_id, artifact_id, event_mission_id?, requested_by_arma_id}
     -> the outcome in the admin's chat and in TBD_AdminAudit
```

`TBD_DeployableMissionList` holds the platform's answer in its order (by title); the list belongs
to the server, not to a world, and `TBD_FrameworkManager` refreshes it on entering `LOBBY`. The relay
sends the admin's `arma_id` from `TBD_PlayerIdentity`; the platform decides whether that identity
belongs to a platform administrator and validates the pick as it validates a website deployment.
Picking the mission this world runs for an event names the running event mission, so its seats and
reservations are kept. The relay restarts nothing: an accepted deployment runs when its
[fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) does, as a `load_mission` in
`apps/mod/tbd-framework/Scripts/Game/TBD/API/FleetCommands/` or a host restart. A refusal is told in
words for its `details.code` (`IDENTITY_NOT_LINKED`, `NOT_AN_ADMINISTRATOR`,
`DEPLOYMENT_IN_PROGRESS`, `ARTIFACT_NOT_APPROVED`, `EVENT_MISSION_NOT_ON_SERVER`, `SERVER_INACTIVE`,
`MODPACK_MISMATCH`, `TERRAIN_NOT_RUNNABLE`, `ORBAT_ARTIFACT_MISMATCH` and others). The browser
payload is capped at 100 lines (`MAX_LIST_LINES`); numbering still covers the whole list.

`TBD_MissionBrowser` registers the `TBD_BrowserContext` keys on the local client:
`TBD_MissionCycle` (F6), `TBD_MissionLoad` (F7), `TBD_AdminMenu` (F8, `TBD_AdminClient.Toggle`) and
`TBD_MissionSelector` (F9, `TBD_MissionSelectorScreen.Toggle`). Its controller block also carries
the admin screen's RPCs, whose logic stays in
`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`.

`TBD_MissionSelectorData` holds the Mission Selector screen's models (`TBD_TerrainInfo`,
`TBD_MissionMode`, `TBD_MissionVersion`, `TBD_MissionMod`, `TBD_MissionAsset`,
`TBD_MissionObjective`, `TBD_MissionFactionSummary`, `TBD_MissionSummary`) and its read surface
`TBD_MissionCatalog`, which `Get()` builds from `TBD_MissionSelectorMock` in
`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/` until something calls `Set()`; no script does,
so the screen shows mock missions, separate from the platform's deployable list above.
`TBD_SessionSelection` keeps the screen's last pick for the lobby and briefing titles.

## Authority

- Server: `TBD_DeployableMissionList` and `TBD_MissionDeploymentRelay` (`@authority server`), and
  the server halves of the RPCs, each of which checks the admin list
  (`SCR_PlayerListedAdminManagerComponent`) itself or through `TBD_AdminService` before acting.
- Client: the key listeners, the cached list lines, the models and the screen.
- Owner: the replies below, delivered to the requesting client only.
- RPCs, all on the modded `SCR_PlayerController`, as their `@rpc` tags state:
  - `TBD_RpcAsk_MissionList`, `TBD_RpcAsk_SelectMission(int)`, `TBD_RpcAsk_AdminSnapshot` and
    `TBD_RpcAsk_AdminAction(int, int)`: Reliable, Server;
  - `TBD_RpcDo_ReceiveMissionList(string)`, `TBD_RpcDo_AdminSnapshot(string)`,
    `TBD_RpcDo_AdminActionResult(string, bool)` and `TBD_RpcDo_OpenAdminMenu`: Reliable, Owner.
  On a listen host the admin snapshot and action requests run directly, without an RPC.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_GameRuntimeHttp`, `TBD_GameRuntimeAnswer` and `TBD_PlayerIdentity` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; `TBD_PlayerChat` and `TBD_Log` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`; `TBD_DeployedMission` (the running deployment)
  in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; the admin scripts in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`; the mock in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`; the key bindings in
  `apps/mod/tbd-framework/Configs/System/Actions/` and
  `apps/mod/tbd-framework/Configs/System/ActionContext/TBD_BrowserContext.conf`. Over HTTP,
  `GET /api/v1/game-runtime/missions` and `POST /api/v1/game-runtime/deployments` of
  `apps/website/api_v2/src/missions/`.
- Used by: `TBD_AdminCommands` (`#tbd missions`, `#tbd mission`, `#tbd refresh`, `#tbd backend`)
  and `TBD_AdminClient` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/`;
  `TBD_FrameworkManager`, which refreshes the list; `TBD_LobbyCatalog` and `TBD_BriefingCatalog`,
  which read `TBD_SessionSelection`.
- Rules: every server half of an RPC takes the caller from `GetPlayerId()` and checks the admin
  list; the relay never restarts anything itself; the request and list shapes follow
  `contracts_v2/definitions/mission-deployment.schema.json`
  (`RelayedDeploymentRequest`, `DeployableMissionList`); lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Missions domain](/apps/website/api_v2/src/missions/README.md) — the deployable list and the
  relayed deployment request on the API side
- [Mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
  — the screen as built, its
  data, design target, open work and decisions
- [Mission selection design references](/documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/README.md)
  — the Stitch mockup sets and the Arma 3 captures the screen started from
