# Briefing screen navigation

The two navigation panels of the Briefing screen, the modes and pages they select, and the table
that maps a page to its width and its page object.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/Navigation/
├── TBD_BriefingNav.c               the ten topic items, page widths and `CreatePage`
├── TBD_BriefingPrimaryNav.c        the primary navigation panel: Map, Briefing, Players, Markers
├── TBD_BriefingTopicNav.c          the topic navigation panel: ten topics in three groups
├── TBD_EBriefingMode.c             the four modes, in primary navigation order
├── TBD_EBriefingPage.c             the ten pages, in topic navigation order
├── TBD_PrimaryNavItemComponent.c   one primary navigation item, bound to its layout
└── TBD_TopicNavItemComponent.c     one topic navigation item, bound to its layout
```

## How it works

`TBD_BriefingPrimaryNav` builds `TBD_PrimaryNav.layout` in LeftDock with four
`TBD_PrimaryNavItemComponent` items; the Players item carries the BLUFOR and OPFOR slotted counts
as chips. `TBD_BriefingTopicNav` builds `TBD_TopicNav.layout` in CenterDock with one
`TBD_TopicNavItemComponent` per `TBD_BriefingNav.TopicItems` entry, a separator before Friendly
Assets and before Objectives. Each panel raises `GetOnSelected()` with (nav, index), the index a
`TBD_EBriefingMode` or a `TBD_EBriefingPage`; `TBD_BriefingScreen` switches on it and asks
`TBD_BriefingNav` for the page width and the page object.

## Authority

- Server: nothing here.
- Client: everything; the panels run on the local player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_UIInteractive`, `TBD_UILayouts`, `TBD_UITheme`, `TBD_UIIcons`,
  `TBD_ChipComponent` and `TBD_NavItemData` in `mod/tbd-framework/Scripts/Game/TBD/UI/`;
  `TBD_PlayersCatalog` in `mod/tbd-framework/Scripts/Game/TBD/Session/Players/`; the pages in
  `mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/Pages/`.
- Used by: `TBD_BriefingScreen` in `mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`.
- Rules: `TBD_PrimaryNavItemComponent` and `TBD_TopicNavItemComponent` are named by their item
  layouts and keep their class names; enum order is item order; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.
