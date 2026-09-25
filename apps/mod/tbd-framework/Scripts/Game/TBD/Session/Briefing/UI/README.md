# Briefing screen

The Briefing tab of the pre-game screens: a full-screen map with a primary navigation, a topic
navigation of ten briefing pages, and the page itself, plus the Ready & Continue action that deploys
the player. The pages read `TBD_BriefingCatalog`, which serves mock data.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/
├── TBD_BriefingMarkersPanel.c  the Markers mode: a plan dropdown and a Load Plan button that logs
├── TBD_BriefingNav.c           the mode and page enums, the two item tables, page widths, `CreatePage`
├── TBD_BriefingPage.c          the page base: fill panel, scroll list, chips, 3D previews, Locate
├── TBD_BriefingPageAssets.c    the Assets and Uniforms pages, friendly and enemy
├── TBD_BriefingPageComms.c     the Frequencies page and the read-only ORBAT page
├── TBD_BriefingPageInfo.c      the Objectives, Rules, Background and Parameters pages
├── TBD_BriefingPrimaryNav.c    the primary navigation panel with slotted-count chips on Players
├── TBD_BriefingScreen.c        `TBD_BriefingScreen`: map, docks, modes, pages, the ready action
└── TBD_BriefingTopicNav.c      the topic navigation panel: ten topics in three groups
```

## How it works

`TBD_BriefingScreen` extends `TBD_DockScreen` over a full-screen `SCR_MapEntity` and opens through
`TBD_MenuStack` on the `TBD_UIBriefing` preset, which the file's `modded enum ChimeraMenuPreset`
adds and `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds to
`apps/mod/tbd-framework/UI/layouts/Session/Briefing/TBD_BriefingScreen.layout`. The primary
navigation sits in LeftDock and selects a `TBD_EBriefingMode` (`SetMode`); the topic navigation sits
in CenterDock and selects a `TBD_EBriefingPage` (`ShowPage`), whose page `TBD_BriefingNav.CreatePage`
builds in RightDock at the page's width. PLAYERS is a mode: `TBD_PlayersPanel` opens in `WideDock`
beside the primary navigation with the map live behind it, and Markers is a mode with
`TBD_BriefingMarkersPanel`. `LocateOnMap(x, z)` pans the map for every Locate button.

Every page extends `TBD_BriefingPage` and reads `TBD_BriefingCatalog.Get()`. The ORBAT page reuses
the lobby's `TBD_LobbyRosterPanel` read-only with Locate buttons, beside a `TBD_KitInspectorPanel`.
The Assets page renders each vehicle type with its specifications, ammunition and inventory, and the
Uniforms page renders one card per faction with a 3D rifleman preview; the enemy pages use the same
builders in the enemy tint.

The bottom bar holds Lock Lobby, which flips its label and logs only, and Ready & Continue, which
calls `TBD_BriefingClient.ReportReady` and `TBD_SpawnClient.Request`: the server's
`TBD_SpawnManager.DeployOnReady` deploys the player, and on a deployed answer the screen closes
every menu through `TBD_MenuStack.CloseAll`. Load Plan logs the chosen plan id; no plan store exists.

## Authority

- Server: nothing here.
- Client: everything; the screen, navigation and pages run on the local player's machine.
- Owner: nothing.
- RPCs: none in this folder; the ready report and the deploy request travel through
  `TBD_BriefingController` and `TBD_SpawnClient`.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_BriefingCatalog` and `TBD_BriefingClient` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/`; `TBD_LobbyCatalog`,
  `TBD_LobbyRosterPanel` and `TBD_KitInspectorPanel` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/`; `TBD_PlayersCatalog` and
  `TBD_PlayersPanel` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/`; `TBD_SpawnClient`
  in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; the shared UI library in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Session/Briefing/`; the engine's `SCR_MapEntity`.
- Used by: `TBD_BriefingController`, which opens and closes the `TBD_UIBriefing` preset on the
  `BRIEFING` stage; `TBD_DockScreen` (the top-bar Briefing tab).
- Rules: pages read only `TBD_BriefingCatalog.Get()`; the map stays open under every mode, so no
  mode pushes a stacked menu; lines added stay ASCII and `cargo xtask mod compile` checks that the
  scripts compile.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the screen as built, its
  data, design target, open work and decisions
- [Briefing design references](/documentation_v2/mod/tbd-framework/UI/briefing/visual_references/README.md)
  — the Stitch mockup sets and the Arma 3 captures the screen started from
