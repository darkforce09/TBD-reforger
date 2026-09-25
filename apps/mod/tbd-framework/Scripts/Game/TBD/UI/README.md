# Framework UI library

The shared view layer of the framework's screens: the menu framework and design tokens, the
reusable components, the live-round HUD, and the mock data the pre-game screens render. Each
screen's own controller lives with its feature under
`apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; nothing here talks to the server except the
HUD's RPC pairs.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/
├── Common/  the shared components: panels, chips, dropdowns, tab strips, the pre-game bars
├── Core/    the menu stack and screen bases, list and button primitives, layouts, theme, icons
├── Hud/     the objective board and capture bar, and task map markers, with their RPC pairs
└── Mock/    the mock catalogs behind the mission selector, lobby, briefing and players screens
```

## How it works

```text
Session/<feature>/UI screens ──derive from──▶ Core/ (TBD_DockScreen, TBD_ShellScreen, TBD_MenuBase)
            │ mount                                   │ names every layout, paints every colour
            ▼                                         ▼
      Common/ components ────────────────────▶ apps/mod/tbd-framework/UI/layouts/
            ▲
Session/<feature> catalogs ──Get() until Set()──▶ Mock/
Gamemode/Objectives ──owner RPC──▶ Hud/
```

`Core/` is the foundation: `TBD_MenuStack` keeps one screen per menu preset and gives input and
focus to the top one, `TBD_UILayouts` names every layout resource once, and `TBD_UITheme` turns
every colour token into what the engine paints. `Common/` components bind the shared
sub-layouts and paint over the ground they sit on. `Hud/` is the one part fed from the server,
by the objective and task runners in
`apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`. `Mock/` supplies the pre-game
catalogs until live ones are set. The post-game end and debrief screens and the per-feature
panels live under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/` and use this library.

## Authority

- Server: `Hud/` builds and sends each player's snapshot; `TBD_UILayouts.Create` returns null on
  a dedicated server, which has no workspace.
- Client: every screen, component, mock catalog and HUD painter.
- Owner: each HUD snapshot goes to its own player's controller.
- RPCs: four, all Reliable on the modded `SCR_PlayerController` in `Hud/`: the objective board
  and the task snapshot, each an ask to the Server and an answer to the Owner.
- Replicated properties: none.

## Boundaries

- Depends on: the layouts and textures in `apps/mod/tbd-framework/UI/`; the menu presets in
  `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`; the objective and task runners in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`; the marker icons in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/`; the catalog classes in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the engine's menu and widget API.
- Used by: the screens and panels under `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the
  objective and task runners, which push the HUD; the layouts that attach the handlers by class.
- Rules: `UI/` holds view code only, and wire code stays in the feature modules; a screen reads a
  catalog or a client cache, never the [mission](/documentation_v2/glossary.md#mission) document or
  a service; layout paths are named once, in `TBD_UILayouts`; colour is a `TBD_UITheme` token or a
  `TBD_EUITint`; lines added to a script stay ASCII, and `cargo xtask mod compile` checks that the
  scripts compile.

## Related documentation

- [Mod UI documentation](/documentation_v2/mod/tbd-framework/UI/README.md) — where each screen's
  scripts and layouts go, the dock shell, and the specification of each in-game screen
