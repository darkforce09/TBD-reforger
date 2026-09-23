# Session/MissionSelector

Mission selection for server administrators: the missions the platform lets this server deploy, and in-game deployment requests relayed to the platform.

### Roles & Responsibilities
- `TBD_MissionBrowser.c`: `modded class SCR_PlayerController` providing client<->server RPC
  transports for admins: the numbered mission list (`TBD_MissionCycle` steps through it) and a
  deployment request for the highlighted number (`TBD_MissionLoad`), answered in chat. Binds F6 / F9
  to `TBD_MissionSelectorScreen.Toggle()`.
- `TBD_DeployableMissionList.c`: `GET /api/v1/game-runtime/missions` (machine credential) - every
  live mission whose approved artifact this server can run, in the platform's order (by title),
  numbered from 1: `n) Title [terrain]`, marked `(running)` for the mission this world runs.
  Refreshed on LOBBY and by `#tbd refresh`.
- `TBD_MissionDeploymentRelay.c`: `POST /api/v1/game-runtime/deployments` {mission_id, artifact_id,
  event_mission_id?, requested_by_arma_id} for the selected number, with the admin's game identity.
  Tells the admin the outcome in chat: accepted (the server restarts when the platform's deployment
  command runs), or the refusal in words (IDENTITY_NOT_LINKED, NOT_AN_ADMINISTRATOR,
  DEPLOYMENT_IN_PROGRESS, ARTIFACT_NOT_APPROVED, EVENT_MISSION_NOT_ON_SERVER, MODPACK_MISMATCH,
  TERRAIN_NOT_RUNNABLE, ORBAT_ARTIFACT_MISMATCH, ...). Never restarts anything itself. Selecting the
  mission this world runs for an event keeps its event mission.
- `TBD_MissionSelectorData.c`: plain models (`TBD_TerrainInfo`, `TBD_MissionMode`,
  `TBD_MissionVersion`, `TBD_MissionMod`, `TBD_MissionAsset`, `TBD_MissionObjective`,
  `TBD_MissionFactionSummary`, `TBD_MissionSummary`) and the read surface `TBD_MissionCatalog`
  (`Get()` / `Set()`, terrain + mode + mission queries with computed counts). Mock-filled today
  by `UI/Mock/TBD_MissionSelectorMock.c`; a future `TBD_MissionSelectorClient` hands `Set()` a
  live catalog.
- `UI/TBD_MissionSelectorScreen.c`: `TBD_MissionSelectorScreen : TBD_DockScreen` — mounts the
  three columns into the `TBD_MissionSelector.layout` shell, wires terrain → browser → inspector,
  owns the `Select Scenario` bottom action, `Toggle()` through `TBD_MenuStack`
  (preset `TBD_UIMissionSelector`). Holds the `modded enum ChimeraMenuPreset` block.
- `UI/TBD_TerrainSelectorPanel.c`: `TBD_TerrainRowComponent` (pooled row) +
  `TBD_TerrainSelectorPanel` (controller: search, selection, `GetOnSelected()(panel, terrainKey)`).
- `UI/TBD_ScenarioBrowserPanel.c`: `TBD_MissionCardComponent` (pooled card) +
  `TBD_ScenarioBrowserPanel` (controller: terrain ∩ modes ∩ query filter, computed `N AVAILABLE`,
  Modes multi-select dropdown, `GetOnSelected()(panel, missionId)`).
- `UI/TBD_MissionInspectorPanel.c`: controller of the right column — hero, version dropdown, and
  four `TBD_Panel` cards (modset grid, summary, ORBAT columns, objective columns) rebuilt per
  mission from Common primitives.

### Call Flow & Contracts
`TBD_LobbyStage` raises it automatically on the LOBBY stage (first screen of a round; F6 / F9 also toggle it via `TBD_MissionSelectorScreen.Toggle()`) → `TBD_MenuStack.Open(TBD_UIMissionSelector)` →
`OnScreenOpen` mounts bars + panels → user picks terrain → browser refilters → user picks card →
inspector rebinds → `Select Scenario` logs `[TBD][selector] SELECT SCENARIO …` (the wire step hands
this to `TBD_MissionBrowser`'s deployment request; nothing in the screen changes when it does) -> the
server relays it to the platform (`TBD_MissionDeploymentRelay`), which runs the deployment through a
fleet command: `load_mission` on this runtime (`API/FleetCommands/`) or a host restart.
