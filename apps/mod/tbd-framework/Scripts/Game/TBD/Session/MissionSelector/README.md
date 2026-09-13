# Session/MissionSelector

Scenario selection and terrain rotation interface for server administrators.

### Roles & Responsibilities
- `TBD_MissionBrowser.c`: `modded class SCR_PlayerController` providing client<->server RPC
  transports for admins to query mission lists, cycle scenarios, and initiate loads. Binds F6 / F9
  to `TBD_MissionSelectorScreen.Toggle()`.
- `TBD_ScenarioRouter.c`: Maps mission terrain slugs (e.g. `"everon"`) to their corresponding
  `.conf` scenario headers and manages addon GUID transitions.
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
this to `TBD_MissionBrowser`'s load request; nothing in the screen changes when it does) → server
authority triggers world change via `TBD_ScenarioRouter` or restarts scenario.
