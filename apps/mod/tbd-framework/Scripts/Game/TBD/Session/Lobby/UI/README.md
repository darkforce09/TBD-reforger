# Lobby screen

The Lobby tab of the pre-game screens: factions on the left, the squads and seats of the chosen
faction in the middle, and a kit inspector with a 3D preview of the chosen seat on the right. It
reads `TBD_LobbyCatalog`, which serves mock data, so its claims and toggles stay on this client.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/
├── Kit/                               the KIT INSPECTOR column and its 3D preview
├── Roster/                            the ROLES column: squad cards and seat rows
├── PauseMenuUI.c                      modded `PauseMenuUI`: "Change slot" opens the lobby mid-round
├── TBD_LobbyFactionPanel.c            the FACTIONS column: pooled faction rows with seat counts
├── TBD_LobbyFactionRowComponent.c     one faction row: name, role chip, claimed count
└── TBD_LobbyScreen.c                  `TBD_LobbyScreen`: dock wiring, Lock and Ready
```

## How it works

```text
TopDock     TBD_SessionTopBar                            mission title, tabs
LeftDock    TBD_LobbyFactionPanel  --GetOnSelected(faction)-->
CenterDock  TBD_LobbyRosterPanel   --GetOnSelected(slotKey)-->
RightDock   TBD_KitInspectorPanel  (hosts TBD_KitPreviewComponent)
BottomDock  TBD_SessionBottomBar                         [ Lock Lobby ] [ Ready & Continue ]
```

`TBD_LobbyScreen` extends `TBD_DockScreen` and opens through `TBD_MenuStack` on the `TBD_UILobby`
preset, which `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Screens/ChimeraMenuPreset.c` adds and
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds to
`apps/mod/tbd-framework/UI/layouts/Session/Lobby/TBD_LobbyScreen.layout`. It is reached from the
selector's top-bar tab, and from the pause menu: the modded `PauseMenuUI` turns the vanilla
leave-faction button into "Change slot" during `BRIEFING`, `SAFE_START` and `LIVE`, which calls
`TBD_LobbyScreen.OpenFromPause` on the next frame. On open the screen reads
`TBD_LobbyCatalog.Get()`, selects the player's own faction (else the first) and own seat, and wires
faction to [roster](Roster/README.md) to [kit inspector](Kit/README.md). The faction column pools
one `TBD_LobbyFactionRowComponent` per faction, spectators in their own dock, with a
`claimed / seats` chip recomputed on every catalog change. Lock Lobby and Ready & Continue flip
their labels and log `[TBD][lobby] LOCK LOBBY -> ...` and `[TBD][lobby] READY -> ...`; neither
reaches the server.

## Authority

- Server: nothing.
- Client: everything; the screen, panels and preview run on the local player's machine
  (`@authority client` on `OpenFromPause` and the pause-menu hook).
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the pause-menu hook reads the replicated stage from
  `TBD_FrameworkManager`.

## Boundaries

- Depends on: `TBD_LobbyCatalog` and its models in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/Catalog/`; `TBD_DockScreen`,
  `TBD_MenuStack`, `TBD_UILayouts`, `TBD_UITheme` and `TBD_UIInteractive` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; the shared primitives in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Session/Lobby/`; the vanilla `PauseMenuUI`.
- Used by: `TBD_DockScreen` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/` (the top-bar Lobby
  tab), which opens the `TBD_UILobby` preset, and `TBD_LobbyStage`, which closes it after `LOBBY`;
  `TBD_BriefingOrbatPage` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/Pages/`,
  which reuses `TBD_LobbyRosterPanel` and `TBD_KitInspectorPanel`.
- Rules: the screen reads seats and kits only through `TBD_LobbyCatalog.Get()`; the screen owns
  wiring and the panels own their widgets; `TBD_LobbyScreen` and `TBD_LobbyFactionRowComponent`
  keep their class names (configs and layouts reference them); lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Lobby specification](/documentation/apps/mod/tbd-framework/UI/lobby/lobby_specification.md) — the screen as built, its
  data, design target, open work and decisions
- [Lobby design references](/documentation/apps/mod/tbd-framework/UI/lobby/visual_references/README.md)
  — the Stitch mockup sets and the Arma 3 captures the screen started from
