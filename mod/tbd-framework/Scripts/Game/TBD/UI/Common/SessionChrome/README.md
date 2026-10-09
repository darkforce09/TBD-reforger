# Pre-game session bars

The two bars every pre-game dock screen wears: the top bar with the title, the Scenario Browser,
Lobby and Briefing tabs, the viewer's identity and the player count, and the bottom action bar with
one primary action.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/UI/Common/SessionChrome/
├── TBD_ESessionTab.c       TBD_ESessionTab: the three pre-game screens in top-bar order
├── TBD_SessionBottomBar.c  TBD_SessionBottomBar: the pre-game action bar, one primary action
└── TBD_SessionTopBar.c     TBD_SessionTopBar: title, screen tabs, identity and count; TBD_SessionIdentity
```

## How it works

`TBD_DockScreen` mounts both bars at open: `TBD_SessionTopBar` from
`Session/Shared/TBD_SessionTopBar.layout` into `TopDock` and `TBD_SessionBottomBar` from
`Session/Shared/TBD_SessionBottomBar.layout` into `BottomDock`. The top bar creates a
`TBD_TabStripComponent` with the three session tabs, shows the screen's title (a screen name upper
case, or a mission id as-is), echoes its `TBD_ESessionTab` and shows the `TBD_SessionIdentity` the
screen returns; a tab click fires `GetOnTabSelected()(bar, tab)`, and `TBD_DockScreen` decides
which preset it opens. The bottom bar holds no buttons of its own: a screen adds each action with
`AddAction(id, label, primary, left)`, which creates `Common/TBD_Button.layout`, and a click fires
`GetOnAction()(bar, id)`. A second primary action demotes the first and logs a WARNING.

## Authority

- Server: nothing.
- Client: everything; the bars run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_TabStripComponent` and `TBD_NavItemData` in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Common/Inputs/`; `TBD_ChipComponent` in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Common/Layout/`; `TBD_UIButton` in
  `mod/tbd-framework/Scripts/Game/TBD/UI/Core/Controls/`; `TBD_UILayouts`, `TBD_UITheme` and
  `TBD_UIIcons` in `mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/`; the layouts in
  `mod/tbd-framework/UI/layouts/Session/Shared/`.
- Used by: `TBD_DockScreen` in `mod/tbd-framework/Scripts/Game/TBD/UI/Core/Screens/`; the
  mission selector, lobby and briefing screens under
  `mod/tbd-framework/Scripts/Game/TBD/Session/`, which add actions and return their tab and
  identity; the mock catalogs in `mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`, which build
  `TBD_SessionIdentity`.
- Rules: `TBD_SessionTopBar` and `TBD_SessionBottomBar` are class names the layouts name; the
  `TBD_ESessionTab` order is the tab order; one primary action per screen; lines added to a script
  stay ASCII, and `cargo xtask mod compile` checks that the scripts compile.
