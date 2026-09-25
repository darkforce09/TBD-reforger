# Mission Selector screen

The Scenario Browser tab of the pre-game screens: terrains on the left, the missions of the chosen
terrain in the middle, and an inspector of the chosen mission on the right. It reads
`TBD_MissionCatalog`, which serves mock data, and its Select Scenario action records the choice
for the screens that follow without requesting a deployment.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/
├── TBD_MissionInspectorPanel.c  right column: hero, versions, modset, summary, ORBAT, objectives
├── TBD_MissionSelectorScreen.c  `TBD_MissionSelectorScreen`: dock wiring, Select Scenario, preset
├── TBD_ScenarioBrowserPanel.c   centre column: pooled mission cards, search and the Modes filter
└── TBD_TerrainSelectorPanel.c   the left column: pooled terrain rows with search and selection
```

## How it works

```text
TopDock     TBD_SessionTopBar                                   "SCENARIO BROWSER", tabs
LeftDock    TBD_TerrainSelectorPanel   --GetOnSelected(terrain)-->
CenterDock  TBD_ScenarioBrowserPanel   --GetOnSelected(mission)-->
RightDock   TBD_MissionInspectorPanel  --GetOnVersionChanged-->
BottomDock  TBD_SessionBottomBar                                 [ Select Scenario ]
OverlayDock the Modes and version popovers
```

`TBD_MissionSelectorScreen` extends `TBD_DockScreen` and opens through `TBD_MenuStack` on the
`TBD_UIMissionSelector` preset, which the file's `modded enum ChimeraMenuPreset` adds and
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds to
`apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/TBD_MissionSelector.layout`. `Toggle()`
raises or drops it. On open it reads `TBD_MissionCatalog.Get()`, mounts the three panels and wires
them with their invokers, then selects the first terrain. The terrain and browser panels pool their
rows and cards (`TBD_TerrainRowComponent`, `TBD_MissionCardComponent`) and rebind them on each
search keystroke; the browser filters by terrain, checked modes and the query, and computes its
`N AVAILABLE` count and per-mode counts from the catalog. The inspector rebuilds its four cards per
mission from the shared primitives. Select Scenario stores the mission and version in
`TBD_SessionSelection`, which the lobby and briefing titles read, and logs
`[TBD][selector] SELECT SCENARIO <title> (<id>) on <terrain>, version <label>`.

## Authority

- Server: nothing.
- Client: everything; the screen and its panels run on the local player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_MissionCatalog`, the mission models and `TBD_SessionSelection` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`; `TBD_DockScreen`,
  `TBD_MenuStack`, `TBD_UILayouts` and `TBD_UITheme` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; the shared primitives in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/`.
- Used by: `TBD_MissionBrowser` (the F9 key calls `Toggle()`) and `TBD_LobbyStage` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/`, which raises the screen on `LOBBY`; the
  preset entry in `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`.
- Rules: the screen reads missions only through `TBD_MissionCatalog.Get()`; the screen owns wiring
  and the panels own their widgets; sources stay ASCII and `cargo xtask mod compile` checks that
  they compile.

## Related documentation

- [Mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
  — the Mission Selector's design target
