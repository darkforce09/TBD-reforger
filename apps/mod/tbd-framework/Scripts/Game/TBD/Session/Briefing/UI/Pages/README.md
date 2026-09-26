# Briefing screen pages

The ten topic pages of the Briefing screen and the base they share. Every page draws from
`TBD_BriefingCatalog`; the enemy Assets and Uniforms pages reuse the friendly builders in the
enemy tint.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/Pages/
├── TBD_BriefingAssetsPage.c        vehicle types, specifications and each vehicle, either side
├── TBD_BriefingBackgroundPage.c    the lore, one inset paragraph each
├── TBD_BriefingFrequenciesPage.c   the long-range command net and the short-range squad nets
├── TBD_BriefingObjectivesPage.c    the time limit and numbered objective cards with Locate
├── TBD_BriefingOrbatPage.c         the lobby roster read-only beside the kit inspector
├── TBD_BriefingPage.c              the base: fill panel, scroll list, chips, previews, Locate, cells
├── TBD_BriefingParametersPage.c    icon, label and mono value rows
├── TBD_BriefingRulesPage.c         collapsible groups of numbered rules
└── TBD_BriefingUniformsPage.c      one card per uniform with a 3D doll, either side
```

## How it works

`TBD_BriefingNav.CreatePage` creates a page and `TBD_BriefingScreen.ShowPage` builds it into the
page column. `TBD_BriefingPage.Build` mounts a fill panel with the page's title, icon and badges and
a scroll list, then calls the subclass's `Fill`; the ORBAT page returns false from `UsesPanel()` and
mounts `TBD_OrbatPage.layout` into the dock instead. The base tracks every 3D preview and Locate
button and releases them in `Destroy`; a Locate button pans through the screen's
`TBD_BriefingMapLauncher`.

## Authority

- Server: nothing here.
- Client: everything; the pages run on the local player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_BriefingCatalog` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/Catalog/`; `TBD_LobbyCatalog`,
  `TBD_LobbyRosterPanel` and `TBD_KitInspectorPanel` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/`; the shared UI library in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Session/Briefing/`.
- Used by: `TBD_BriefingNav` and `TBD_BriefingScreen` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`.
- Rules: a page reads only `TBD_BriefingCatalog` and the lobby catalog; previews are destroyed
  before their widgets are cleared; lines added stay ASCII and `cargo xtask mod compile` checks that
  the scripts compile.
