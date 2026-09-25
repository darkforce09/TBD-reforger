# Briefing screen layouts

The layouts of the Briefing tab: a dock shell laid over the full-screen map, the two navigation
panels, and the rows, cards and panels the briefing pages mount. `TBD_BriefingScreen` opens the
shell and fills its docks; every page reads mock data from `TBD_BriefingCatalog`.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/Briefing/
├── TBD_AssetPreview.layout*    Vehicle Info box: 168 px 3D preview over a grid, with a caption
├── TBD_BriefingScreen.layout*  the shell: vanilla map, map tool menu and empty docks
├── TBD_FreqRow.layout*         one radio net: name, frequency chip and auxiliary channel line
├── TBD_MarkersPanel.layout*    Markers mode panel: caption, plan dropdown and Load Plan docks
├── TBD_OrbatPage.layout*       ORBAT page: a 500 px roster dock beside a kit dock
├── TBD_PrimaryNav.layout*      the primary navigation's glass panel, sized to its items
├── TBD_PrimaryNavItem.layout*  one primary item: accent bar, 32 px icon box, label, badges
├── TBD_TopicNav.layout*        the topic navigation's 340 px glass panel, sized to its items
├── TBD_TopicNavItem.layout*    one topic: optional group rule above, icon and label
└── TBD_UniformCard.layout*     one faction uniform: name, 240 px 3D preview, chips, camo
```

Each `.layout` sits beside its `.layout.meta`, so every line covers the pair.

## How it works

`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds the `TBD_UIBriefing` preset to
`TBD_BriefingScreen.layout` and the `TBD_BriefingScreen` class
(`apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/TBD_BriefingScreen.c`), a
`TBD_DockScreen`. The shell's `MapFrame` includes the vanilla
`{0651202E9F2646DE}UI/layouts/Map/Map.layout`, so the map fills the screen and never resizes;
every panel is an overlay above it and the shell has no `Backdrop`. `ToolMenu` is the frame the
vanilla map tool menu (`SCR_MapToolMenuUI`) finds by that name, top right under the top bar.

```text
MapFrame (the vanilla map, full screen)
┌ TopDock 56 ── TBD_SessionTopBar ──────────────────────────────────────────────────────┐
│ LeftDock 288      CenterDock 340 (x 300)      RightDock (x 652) > PageHost > PageFrame │
│ primary nav       topic nav / markers panel   the page, at the page's width           │
│                   WideDock (x 300, 800 wide): the players panel                       │
└ BottomDock 64 ── TBD_SessionBottomBar ────────────────────────────────────────────────┘
OverlayDock (full screen, last): dropdown menus
```

The primary navigation picks a mode. In the briefing mode the topic navigation fills CenterDock
and the chosen page mounts into `PageFrame`; the page width is `PageHost`'s width override, set
per page from `TBD_BriefingNav.PageWidth`: Frequencies 448,
[ORBAT](/documentation_v2/glossary.md#orbat) 1200, Friendly and Enemy Assets 576, Friendly and
Enemy Uniforms 768, Objectives 440, Rules 480, Background 440, Parameters 440.
The Markers mode puts the markers panel in CenterDock, and the Players mode builds
`apps/mod/tbd-framework/UI/layouts/Session/Shared/TBD_PlayersPanel.layout` into WideDock. While
the cursor is over a visible panel (its `PanelBorder`) or the page column (`PageHost`), the screen
withholds `MapContext`, so the mouse wheel scrolls the panel instead of zooming the map.

The pages build mostly from `apps/mod/tbd-framework/UI/layouts/Common/`: a fill panel, a scroll
list, sections, numbered cards, key-value rows, captions, stat-cell grids and inset text. The
layouts here cover what no other screen needs. The ORBAT page mounts the lobby's roster column and
kit inspector from `apps/mod/tbd-framework/UI/layouts/Session/Lobby/` into its two docks, read
only. Enemy pages reuse the friendly builders painted in the `TBD_EUITint.OPFOR` tint.

| Layout | Widgets the handler binds |
|---|---|
| `TBD_BriefingScreen` | `MapFrame`, `ToolMenu`, `WindowFrame`, `TopDock`, `LeftDock`, `CenterDock`, `RightDock` > `PageHost` (a size layout) > `PageFrame`, `WideDock`, `BottomDock`, `OverlayDock` |
| `TBD_FreqRow` | `Border`, `Background`, `Name`, `FreqChipDock`, `AuxRule`, `AuxLabel`, `AuxValue` |
| `TBD_OrbatPage` | `RosterDock` (500 px), `KitDock` (the rest) |
| `TBD_AssetPreview` | `PreviewBorder`, `PreviewBG`, `GridClip`, `GridImage`, `Preview`, `Label` |
| `TBD_UniformCard` | `Border`, `Background`, `Name`, `HeaderRule`, `PreviewBorder`, `PreviewBG`, `GridImage`, `Preview`, `Label`, `ChipsDock`, `CamoLabel` |
| `TBD_MarkersPanel` | `PanelBorder`, `PanelBG`, `CaptionDock`, `PlanDropdownDock`, `LoadButtonDock` |
| `TBD_PrimaryNav`, `TBD_TopicNav` | `PanelBorder`, `PanelBG`, `Items` |
| `TBD_PrimaryNavItem` | `Border`, `Background`, `AccentSize`, `Accent`, `IconBoxBorder`, `IconBoxBG`, `Icon`, `Label`, `BadgeDock` |
| `TBD_TopicNavItem` | `SeparatorSize`, `Separator`, `Border`, `Background`, `Icon`, `Label` |

## Format

- File type: [Enfusion](/documentation_v2/glossary.md#enfusion) widget layouts (`.layout`), plain
  text, each beside a `.layout.meta` whose `Name` holds
  `{GUID}UI/layouts/Session/Briefing/<file>.layout`. Every `*Border`, `*BG` and `Background` is an
  empty `FrameWidgetClass` dock that the handler fills with a rounded shape (panels 12, rows and
  cards 8, preview boxes 6), and every text widget carries a `FontProperties` block. `Preview` in
  the asset preview and the uniform card is an `ItemPreviewWidgetClass`.
- Resource GUID: `7BD1A7000000XX01` in each `.meta`, from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`: the shell `0D`, then
  `TBD_FreqRow` `3E`, `TBD_OrbatPage` `3F`, `TBD_AssetPreview` `40`, `TBD_UniformCard` `41`,
  `TBD_MarkersPanel` `42`, `TBD_PrimaryNav` `46`, `TBD_PrimaryNavItem` `47`, `TBD_TopicNavItem`
  `48` and `TBD_TopicNav` `49`. The shell's GUID is named by the menu config and never changes.
- Naming: `TBD_<Element>.layout`; a layout another screen reuses moves to `Session/Shared/` or
  `Common/`.
- Adding a layout: take a free block from the ledger, author the layout and its `.meta`, add a
  `BRIEFING_*` constant to `TBD_UILayouts`, and commit both files; the game finds a new path only
  after [Workbench](/documentation_v2/glossary.md#workbench) has rewritten `resourceDatabase.rdb`.

## Referenced by

- `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` names the shell by GUID in the
  `TBD_UIBriefing` preset.
- `TBD_UILayouts` names each layout by GUID and path (`BRIEFING_SCREEN` and the `BRIEFING_*`
  constants); the classes in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/` use
  them:
  - `TBD_BriefingPrimaryNav` and `TBD_BriefingTopicNav` build the two panels and their items,
    which carry the `TBD_PrimaryNavItemComponent` and `TBD_TopicNavItemComponent` handlers;
  - `TBD_BriefingFrequenciesPage` adds a `TBD_FreqRow` per radio net, and
    `TBD_BriefingOrbatPage` mounts `TBD_OrbatPage`;
  - `TBD_BriefingAssetsPage` mounts `TBD_AssetPreview` and `TBD_BriefingUniformsPage` mounts
    `TBD_UniformCard`, each showing a 3D model through `TBD_KitPreviewComponent`;
  - `TBD_BriefingMarkersPanel` mounts `TBD_MarkersPanel`, whose Load Plan button only logs the
    chosen plan.

## Boundaries

- Depends on: the vanilla `Map.layout` and the map's `SCR_MapEntity`; the layouts in
  `apps/mod/tbd-framework/UI/layouts/Common/`, `apps/mod/tbd-framework/UI/layouts/Session/Shared/`
  and `apps/mod/tbd-framework/UI/layouts/Session/Lobby/`; the handler classes named above.
- Used by: the briefing screen in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`,
  through the `TBD_UIBriefing` preset.
- Rules: the map never resizes, so every panel stays an overlay; a page's width comes from
  `TBD_BriefingNav.PageWidth`, never from the dock; the players panel stays a mode inside this
  screen, because a menu pushed on top would close the map; the widget names above are the
  handlers' contract; a layout and its `.meta` are committed together.

## Related documentation

- [Briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md)
  — the screen as built, its
  data, design target, open work and decisions
- [Briefing design references](/documentation_v2/mod/tbd-framework/UI/briefing/visual_references/README.md)
  — the Stitch mockup sets and the Arma 3 captures the screen started from
