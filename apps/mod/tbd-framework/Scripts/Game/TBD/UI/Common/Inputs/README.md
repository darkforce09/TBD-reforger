# Search and tab inputs

The controls a player types into or picks from on the pre-game screens: the search field with its
clear button, and the segmented tab strip with its pooled tabs.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/Inputs/
├── TBD_NavItemComponent.c    TBD_NavItemComponent: one tab; TBD_NavItemData: the data a tab is built from
├── TBD_SearchBoxComponent.c  TBD_SearchBoxComponent: a search field with a clear button
└── TBD_TabStripComponent.c   TBD_TabStripComponent: a row or column of pooled tabs
```

## How it works

`TBD_SearchBoxComponent` sits on the root of `Common/TBD_SearchBox.layout`, so the edit box's
change and the clear button's click reach it by widget identity. Every keystroke and every clear
fires `GetOnChanged()(box, query)`; `GetQuery()` returns the trimmed text, and the static
`Matches(query, haystack)` is the case-insensitive test every list filters with. The border lights
while the edit box has focus.

`TBD_TabStripComponent` sits on `Common/TBD_TabStrip.layout` and builds its tabs from a
`TBD_NavItemData` table with `SetItems`: a `Common/TBD_NavItem.layout` widget is created once per
index and surplus tabs are hidden. Each `TBD_NavItemComponent`, a `TBD_UIInteractive`, reports its
click to the strip, which echoes it as the active tab and fires `GetOnSelected()(strip, index)`;
`SetActive` echoes without firing. The `m_bVertical` attribute picks the `ItemsColumn` container
over `ItemsRow`, and `m_bChrome` draws the strip's own pill background; tabs paint over
`GetItemGround()`.

## Authority

- Server: nothing.
- Client: everything; the controls run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_UIInteractive` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Controls/`;
  `TBD_UILayouts`, `TBD_UITheme` and `TBD_UIIcons` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/`; the layouts
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_SearchBox.layout`,
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_TabStrip.layout` and
  `apps/mod/tbd-framework/UI/layouts/Common/TBD_NavItem.layout`.
- Used by: `TBD_SessionTopBar` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/SessionChrome/`
  (the session tabs); the briefing topic navigation under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/Navigation/`, which builds
  `TBD_NavItemData` tables; the pre-game list panels, which nest the search box; the layouts that
  attach the three handlers by class.
- Rules: `TBD_SearchBoxComponent`, `TBD_TabStripComponent` and `TBD_NavItemComponent` are class
  names the layouts name; filtering belongs to the owner of a search box; lines added to a script
  stay ASCII, and `cargo xtask mod compile` checks that the scripts compile.
