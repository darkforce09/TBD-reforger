# Shared UI components

The handlers behind the reusable sub-layouts every TBD screen is built from: panels, chips,
search boxes, tab strips, dropdowns, key-value rows, section and numbered cards, captions, scroll
lists, and the top and bottom bars of the pre-game screens. They bind widgets and nothing else:
no screen knowledge, no network, no [mission](/documentation_v2/glossary/g_to_m.md#mission) data.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/
├── TBD_Caption.c                TBD_Caption: the uppercase mono label that opens a list section
├── TBD_ChipComponent.c          TBD_ChipComponent: a tinted tag or pill badge
├── TBD_DropdownComponent.c      TBD_DropdownComponent: a trigger with a single- or multi-select menu
├── TBD_KeyValueRowComponent.c   TBD_KeyValueRowComponent: a key chip beside a value, tintable
├── TBD_NumberedCardComponent.c  TBD_NumberedCardComponent: a numbered card with chip, body and footer
├── TBD_PanelComponent.c         TBD_PanelComponent: the glass panel: header, badge, body, footer
├── TBD_ScrollList.c             TBD_ScrollList: a scrolling list that wears the TBD scrollbar
├── TBD_SearchBoxComponent.c     TBD_SearchBoxComponent: a search field with a clear button
├── TBD_SectionComponent.c       TBD_SectionComponent: a collapsible card with badge and action dock
├── TBD_SessionBottomBar.c       TBD_SessionBottomBar: the pre-game action bar, one primary action
├── TBD_SessionTopBar.c          TBD_SessionTopBar: pre-game title, screen tabs, identity and count
└── TBD_TabStripComponent.c      TBD_TabStripComponent, TBD_NavItemComponent: a row or column of tabs
```

## How it works

Each handler is a `ScriptedWidgetComponent` on the root of its layout, found by the widget names
the layout declares:

| Handler | Layout under `apps/mod/tbd-framework/UI/layouts/` | Output invoker |
|---|---|---|
| `TBD_PanelComponent` | `Common/TBD_Panel.layout`, `Common/TBD_PanelFill.layout` | none |
| `TBD_ChipComponent` | `Common/TBD_Chip.layout` | none |
| `TBD_SearchBoxComponent` | `Common/TBD_SearchBox.layout` | `GetOnChanged()(box, query)` |
| `TBD_NavItemComponent`, `TBD_TabStripComponent` | `Common/TBD_NavItem.layout`, `Common/TBD_TabStrip.layout` | `GetOnSelected()(strip, index)` |
| `TBD_KeyValueRowComponent` | `Common/TBD_KeyValueRow.layout` | none |
| `TBD_DropdownComponent` | `Common/TBD_Dropdown.layout`, menu `Common/TBD_DropdownMenu.layout` | `GetOnChanged()(dropdown, tag)` |
| `TBD_SectionComponent` | `Common/TBD_Section.layout` | `GetOnToggled()(section, expanded)` |
| `TBD_NumberedCardComponent` | `Common/TBD_NumberedCard.layout` | none |
| `TBD_SessionTopBar` | `Session/Shared/TBD_SessionTopBar.layout` | `GetOnTabSelected()(bar, tab)` |
| `TBD_SessionBottomBar` | `Session/Shared/TBD_SessionBottomBar.layout` | `GetOnAction()(bar, id)` |

`TBD_Caption` and `TBD_ScrollList` are not handlers: `TBD_Caption.Mount` writes and paints
`Common/TBD_Caption.layout`, and `TBD_ScrollList` (a `Managed` object) mounts
`Common/TBD_ScrollList.layout` into a dock and gives it a `TBD_UIScrollBar`.

A handler arrives either through the layout hierarchy, when a screen layout nests the
sub-layout, or at runtime through a static `Mount(...)` that creates the layout into a dock and
returns the handler (`TBD_ChipComponent`, `TBD_KeyValueRowComponent`, `TBD_DropdownComponent`,
`TBD_SectionComponent`, `TBD_NumberedCardComponent`, `TBD_Caption`, `TBD_ScrollList`). Child
events, such as an edit box's change or a clear button's click, arrive at the root handler and are
matched by widget identity. The static mounts and the list setters take:

| Call | Signature |
|---|---|
| `TBD_ChipComponent.Mount` | `(Widget dock, string text, TBD_EUITint tint, int ground = 0)` |
| `TBD_KeyValueRowComponent.Mount` | `(Widget container)` |
| `TBD_DropdownComponent.Mount` | `(Widget dock, Widget overlayHost, string label, bool multi)` |
| `TBD_SectionComponent.Mount` | `(Widget parent, string title, int ground)` |
| `TBD_NumberedCardComponent.Mount` | `(Widget parent, int number, string title, int ground)` |
| `TBD_Caption.Mount` | `(Widget parent, string text, string trailing = "")`, returns the widget |
| `TBD_ScrollList.Mount` | `(Widget dock, int ground, int inset = 12)` |
| `TBD_TabStripComponent.SetItems` | `(notnull array<ref TBD_NavItemData> items)` |
| `TBD_DropdownComponent.SetItems` | `(notnull array<ref TBD_DropdownItem> items)` |

`TBD_ScrollList` mounts its bar with `TBD_UIScrollBar.Mount`, which ticks at 30 Hz, and its
`Destroy` stops the bar and drops its widget references.

Four conventions hold across the folder:

- Ground: every chrome handler takes `SetGround(opaqueArgb)`, the opaque colour it sits on, and
  paints through `TBD_UITheme.PaintOver`, which composites translucent tokens in sRGB;
  `TBD_PanelComponent.GetGround()`, `TBD_SectionComponent.GetBodyGround()` and
  `TBD_TabStripComponent.GetItemGround()` hand the composited fill to the children.
- Shape: every `*Border` and `*BG` dock is filled with a rounded shape at attach
  (`TBD_UILayouts.MountRounded`), at `RADIUS_PANEL` for panels and bars and `RADIUS_ROW` for rows;
  `TBD_ChipComponent.SetPill(true)` swaps the tag radius for a full pill.
- Interaction: `TBD_NavItemComponent` and `TBD_DropdownComponent` derive from `TBD_UIInteractive`,
  so hover and gamepad focus are one state and one click is the action. Colour comes from
  `TBD_UITheme` with `TBD_EUITint`, icons from `TBD_UIIcons` keys.
- Shrink-wrap or stretch: chips, buttons, nav items and dropdown triggers have a left-aligned
  `AlignableSlot` root and size to their text; panels and search boxes anchor to fill their frame
  dock; rows, sections, numbered cards and captions declare a stretched root, and the section,
  numbered-card and caption mounts create it with `TBD_UILayouts.CreateStretched`, which re-applies
  the stretch inside a layout widget.

The dropdown menu opens into the owning screen's overlay dock (`SetOverlayHost`) under the
trigger; its full-bleed `Scrim` closes it, forwarded by `TBD_DropdownMenuBridge`, and its rows are
a `TBD_ListBox`. The bottom bar holds no buttons of its own: a screen adds each action with
`AddAction()`, which creates `Common/TBD_Button.layout`, and a second primary action demotes the
first. The top bar's tabs are `TBD_ESessionTab` (`SCENARIO_BROWSER`, `LOBBY`, `BRIEFING`);
which screen a tab opens is `TBD_DockScreen`'s decision.

## Authority

- Server: nothing.
- Client: everything; the handlers run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_UILayouts`, `TBD_UITheme`, `TBD_UIIcons`, `TBD_UIInteractive`, `TBD_UIButton`,
  `TBD_ListBox` and `TBD_UIScrollBar` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`; the
  layouts in `apps/mod/tbd-framework/UI/layouts/Common/` and
  `apps/mod/tbd-framework/UI/layouts/Session/Shared/`.
- Used by: `TBD_DockScreen` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`, which mounts
  both bars; the briefing, lobby, mission selector and players screens under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the mock catalogs in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`, which build `TBD_SessionIdentity`; and the
  layouts named in the table, which attach the handlers by class.
- Rules: a handler binds widgets by the names its layout declares, so a renamed widget changes
  both files together; a component that sits on a translucent surface paints over its ground,
  never with engine alpha; a component holds no screen or network logic; lines added to a script
  stay ASCII, and `cargo xtask mod compile` checks that the scripts compile, while how a
  component looks is checked in [Workbench](/documentation_v2/glossary/n_to_z.md#workbench).

## Related documentation

- [Mod UI documentation](/documentation_v2/mod/tbd-framework/UI/README.md) — where each screen's
  scripts and layouts go, and the dock shell the bars belong to
