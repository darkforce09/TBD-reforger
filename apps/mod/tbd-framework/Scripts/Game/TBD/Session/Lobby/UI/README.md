# Lobby screen

The Lobby tab of the pre-game screens: factions on the left, the squads and seats of the chosen
faction in the middle, and a kit inspector with a 3D preview of the chosen seat on the right. It
reads `TBD_LobbyCatalog`, which serves mock data, so its claims and toggles stay on this client.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/
├── TBD_KitInspectorPanel.c    the KIT INSPECTOR column: header, preview card, gear and weapons
├── TBD_KitPreviewComponent.c  the 3D doll wearing the seat's kit, turned and zoomed by the mouse
├── TBD_LobbyFactionPanel.c    the FACTIONS column: pooled faction rows with seat counts
├── TBD_LobbyRosterPanel.c     the ROLES column: squad cards and seat rows to select, claim, release
└── TBD_LobbyScreen.c          `TBD_LobbyScreen`: dock wiring, Lock and Ready, the pause-menu hook
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
preset, which the file's `modded enum ChimeraMenuPreset` adds and
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds to
`apps/mod/tbd-framework/UI/layouts/Session/Lobby/TBD_LobbyScreen.layout`. It is reached from the
selector's top-bar tab, and from the pause menu: the file's `modded class PauseMenuUI` turns the
vanilla leave-faction button into "Change slot" during `BRIEFING`, `SAFE_START` and `LIVE`, which
calls `OpenFromPause`. On open it reads `TBD_LobbyCatalog.Get()` and wires faction to roster to kit
inspector.

In the roster, a click selects a seat and shows its kit, a second click on the selected open seat
claims it, and a second click on your own seat releases it; each goes to `TBD_LobbyCatalog.Claim`
or `Release`. `SetReadOnly(true)` limits clicks to selection and `SetShowLocate(true)` adds a Locate
button to every squad header, raising `GetOnLocate()` with the callsign; the briefing's ORBAT page
uses both. The kit inspector rebuilds its cards per seat from `TBD_LobbyCatalog.GetKit` (GEAR,
WEAPONS, GRENADES, GADGETS, TOOLS, MEDICAL, MISC) and destroys the preview before each rebuild.
`TBD_KitPreviewComponent` asks `TBD_LoadoutPreviewDresser` for a preview entity wearing the kit's
base prefab and loadout, shows it through the `ItemPreviewManagerEntity`, and turns it on a drag
(polled at 30 Hz) and zooms it on the wheel; when anything is missing it captions
"PREVIEW UNAVAILABLE" and logs one WARNING. Lock Lobby and Ready & Continue flip their labels and
log `[TBD][lobby] LOCK LOBBY -> …` and `[TBD][lobby] READY -> …`; neither reaches the server.

## Authority

- Server: nothing.
- Client: everything; the screen, panels and preview run on the local player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none; the pause-menu hook reads the replicated stage from
  `TBD_FrameworkManager`.

## Boundaries

- Depends on: `TBD_LobbyCatalog` and its models in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/`; `TBD_LoadoutPreviewDresser` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/`; `TBD_DockScreen`, `TBD_MenuStack`,
  `TBD_UILayouts`, `TBD_UITheme` and `TBD_UIInteractive` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; the shared primitives in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Session/Lobby/`; the engine's `ItemPreviewManagerEntity`.
- Used by: `TBD_DockScreen` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/` (the top-bar Lobby
  tab), which opens the `TBD_UILobby` preset, and `TBD_LobbyStage`, which closes it after `LOBBY`;
  `TBD_BriefingOrbatPage` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`, which
  reuses `TBD_LobbyRosterPanel` and `TBD_KitInspectorPanel`.
- Rules: the screen reads seats and kits only through `TBD_LobbyCatalog.Get()`; the screen owns
  wiring and the panels own their widgets; the preview is destroyed before its card is cleared;
  lines added stay ASCII and `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) — the
  lobby's design target
