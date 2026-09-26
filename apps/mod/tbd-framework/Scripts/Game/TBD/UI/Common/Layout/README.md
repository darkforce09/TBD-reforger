# Panels, cards, rows and chips

The display building blocks of every pre-game screen: the glass panel, the collapsible section,
the numbered card, the key-value row, the chip, the section caption and the scrolling list.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/Layout/
├── TBD_Caption.c                TBD_Caption: the uppercase mono label that opens a list section
├── TBD_ChipComponent.c          TBD_ChipComponent: a tinted tag or pill badge
├── TBD_KeyValueRowComponent.c   TBD_KeyValueRowComponent: a key chip beside a value, tintable
├── TBD_NumberedCardComponent.c  TBD_NumberedCardComponent: a numbered card with chip, body and footer
├── TBD_PanelComponent.c         TBD_PanelComponent: the glass panel: header, badge, body, footer
├── TBD_ScrollList.c             TBD_ScrollList: a scrolling list that wears the TBD scrollbar
└── TBD_SectionComponent.c       TBD_SectionComponent: a collapsible card with badge and action dock
```

## How it works

The five handlers sit on the roots of their layouts under
`apps/mod/tbd-framework/UI/layouts/Common/` and bind widgets by the names those layouts declare.
`TBD_PanelComponent` serves both `TBD_Panel.layout` (content-sized) and `TBD_PanelFill.layout`
(frame-anchored, for columns). Each paints over the opaque ground it is given with `SetGround`
and hands its own composited fill to its children (`GetGround`, `GetBodyGround`); tint colours
come from `TBD_UITintColours`. The chip, key-value row, section and numbered card have a static
`Mount` that creates the layout into a dock and returns the handler; the section, numbered card and
caption mount stretched to the parent's width.

`TBD_Caption` and `TBD_ScrollList` are not handlers: `TBD_Caption.Mount` creates, writes and paints
`TBD_Caption.layout`, and `TBD_ScrollList` (a `Managed` object) creates `TBD_ScrollList.layout`
into a dock, mounts a `TBD_UIScrollBar` on it and stops the bar in `Destroy`.

## Authority

- Server: nothing.
- Client: everything; the components run in the local UI and never replicate.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_UILayouts`, `TBD_UITheme`, `TBD_UITintColours` and `TBD_UIIcons` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Theme/`; `TBD_UIScrollBar` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/Controls/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Common/`.
- Used by: the briefing pages and panels, the lobby roster and faction panels, the kit inspector,
  the mission selector panels and the players panel under
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`; the dropdown badge in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/Dropdown/` and the top bar in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/SessionChrome/`, which mount chips; the layouts
  that attach the handlers by class.
- Rules: `TBD_ChipComponent`, `TBD_KeyValueRowComponent`, `TBD_NumberedCardComponent`,
  `TBD_PanelComponent` and `TBD_SectionComponent` are class names the layouts name; a component on a
  translucent surface paints over its ground, never with engine alpha; lines added to a script stay
  ASCII, and `cargo xtask mod compile` checks that the scripts compile.
