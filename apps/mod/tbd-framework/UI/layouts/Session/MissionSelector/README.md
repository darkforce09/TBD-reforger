# Mission Selector screen layouts

The layouts of the Scenario Browser tab, where a player picks a terrain, a
[mission](/documentation_v2/glossary/g_to_m.md#mission) and its version: a dock shell, the terrain and
mission column bodies with their pooled rows and cards, and the mission inspector with its photo
hero. `TBD_MissionSelectorScreen` fills the shell from `TBD_MissionCatalog`, which serves mock data.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/
├── TBD_FactionColumn.layout*     one faction column of the ORBAT and objectives cards
├── TBD_MissionCard.layout*       one pooled mission card: tag, terrain, slot count, title
├── TBD_MissionInspector.layout*  right column: photo hero, then a scrolling card stack
├── TBD_MissionSelector.layout*   the shell: backdrop and empty docks
├── TBD_ModGridItem.layout*       one mod of the required modset: sync mark, name, version
├── TBD_ScenarioBrowser.layout*   MISSIONS body: search and Modes docks over a card list
├── TBD_TerrainRow.layout*        one pooled terrain row: accent, icon, title, count, chevron
└── TBD_TerrainSelector.layout*   TERRAINS body: search dock over a scrolling row list
```

Each `.layout` sits beside its `.layout.meta`, so every line covers the pair.

## How it works

`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds the `TBD_UIMissionSelector` preset
to `TBD_MissionSelector.layout` and the `TBD_MissionSelectorScreen` class
(`apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/TBD_MissionSelectorScreen.c`),
a `TBD_DockScreen`. The shell holds a `Backdrop`, a `WindowFrame` inset 16 px, and the docks, in
reference pixels:

| Dock | Geometry | Mounted there | By |
|---|---|---|---|
| `TopDock` | full width, 56 high | the session top bar | `TBD_DockScreen` |
| `LeftDock` | x 0, 320 wide, y 68 to 76 above the bottom | `TBD_PanelFill` with the `TBD_TerrainSelector` body | `TBD_TerrainSelectorPanel` |
| `CenterDock` | x 332, 440 wide | `TBD_PanelFill` with the `TBD_ScenarioBrowser` body | `TBD_ScenarioBrowserPanel` |
| `RightDock` | x 784 to the right edge | `TBD_MissionInspector` | `TBD_MissionInspectorPanel` |
| `BottomDock` | full width, 64 high | the session bottom bar: Select Scenario | `TBD_DockScreen` |
| `OverlayDock` | full screen, last child, hidden while empty | the Modes and version menus | `TBD_DropdownComponent` |

The terrain and browser panels pool their rows and cards, rebinding them on each search keystroke.
The inspector stacks four `TBD_Panel` cards from `apps/mod/tbd-framework/UI/layouts/Common/` in
`CardsContent`: the required modset (`TBD_ModGridItem`s in `TBD_Columns2`), the summary
(`TBD_InsetText`), and the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) and objectives, each a
`TBD_Columns2` pair of faction-tinted `TBD_Panel`s whose body is a `TBD_FactionColumn`, with
`TBD_KeyValueRow`s in its `Rows`.

The hero is a 176 px band at the top of the inspector: `HeroImage` shows the terrain's hero
texture, `HeroDim` darkens it, and `HeroFade` (the bottom 110 px) fades it into the solid colour
the title, author and version rows sit on. Nothing clips an image to an arc, so the inspector
paints clipped quarters of an inverse disc over its top corners: `HeroMaskTL` and `HeroMaskTR` in
the panel border colour inside the hero, and `PanelMaskTL` and `PanelMaskTR` in the backdrop
colour on the panel root. The card stack (`BodyFrame`) starts 16 px below the hero, with no rule
between them. The textures live in `apps/mod/tbd-framework/UI/Textures/TBD/`.

| Layout | Handler | Widgets the handler binds |
|---|---|---|
| `TBD_TerrainSelector` | `TBD_TerrainSelectorPanel` | `SearchStrip`, `SearchDock`, `SearchRule`, `ListFrame`, `Scroll`, `Content`, `ScrollBarDock`, `EmptyState` |
| `TBD_TerrainRow` | `TBD_TerrainRowComponent` | `Border`, `Background`, `Accent`, `Icon`, `Title`, `CountBadgeDock`, `Chevron` |
| `TBD_ScenarioBrowser` | `TBD_ScenarioBrowserPanel` | `ToolRow`, `SearchDock`, `ModesDock`, `ToolRule`, `ListFrame`, `Scroll`, `Content`, `ScrollBarDock`, `EmptyState` |
| `TBD_MissionCard` | `TBD_MissionCardComponent` | `Border`, `Background`, `TagChipDock`, `TerrainText`, `SlotCount`, `PulseDot`, `Title`, `Indicator`, `IndicatorGlyph` |
| `TBD_MissionInspector` | `TBD_MissionInspectorPanel` | `PanelBorder`, `PanelBG`, `Hero`, `HeroImage`, `HeroDim`, `HeroFade`, `HeroMaskTL`/`TR` and their `*Img`, `HeroTitle`, `HeroTagDock`, `AuthorIcon`, `AuthorBy`, `AuthorName`, `AuthorChipDock`, `VersionDock`, `PanelMaskTL`/`TR` and their `*Img`, `BodyFrame`, `Scroll`, `CardsContent`, `ScrollBarDock`, `EmptyState` |
| `TBD_ModGridItem` | `TBD_MissionInspectorPanel` | `Border`, `Background`, `CheckIcon`, `CheckGlyph`, `ModName`, `VersionChipDock` |
| `TBD_FactionColumn` | `TBD_MissionInspectorPanel` | `FactionChipDock`, `RoleText`, `CountText`, `CountLabel`, `HeaderRule`, `Rows` |

## Format

- File type: [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) widget layouts (`.layout`), plain
  text, each beside a `.layout.meta` whose `Name` holds
  `{GUID}UI/layouts/Session/MissionSelector/<file>.layout`. Every `*Border`, `*BG` and
  `Background` is an empty `FrameWidgetClass` dock that the handler fills with a rounded shape
  (columns and the inspector 12, rows, cards and mod items 8); the row `Accent` bar, the rules and
  the lower edge of each panel header stay square; every text widget carries a `FontProperties`
  block.
- Resource GUID: `7BD1A7000000XX01` in each `.meta`, from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`: the shell `0B`, then
  `TBD_TerrainSelector` `1D`, `TBD_ScenarioBrowser` `1E`, `TBD_MissionInspector` `1F`,
  `TBD_ModGridItem` `20`, `TBD_FactionColumn` `21`, `TBD_TerrainRow` `22` and `TBD_MissionCard`
  `23`. The shell's GUID is named by the menu config and never changes.
- Naming: `TBD_<Element>.layout`; a layout another screen needs moves to `Common/`.
- Adding a layout: take a free block from the ledger, author the layout and its `.meta`, add a
  `MISSION_SELECTOR_*` constant to `TBD_UILayouts`, and commit both files; the game finds a new
  path only after [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) has rewritten
  `resourceDatabase.rdb`.

## Referenced by

- `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` names the shell by GUID in the
  `TBD_UIMissionSelector` preset.
- `TBD_UILayouts` names each layout by GUID and path (`MISSION_SELECTOR` and the
  `MISSION_SELECTOR_*` constants); the panels in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/` use them, as the handler
  table shows.

## Boundaries

- Depends on: the layouts in `apps/mod/tbd-framework/UI/layouts/Common/` and the session bars in
  `apps/mod/tbd-framework/UI/layouts/Session/Shared/`; the hero textures in
  `apps/mod/tbd-framework/UI/Textures/TBD/`; the handler classes named above.
- Used by: the Mission Selector screen only.
- Rules: the widget names above are the handlers' contract; the shell keeps its GUID and path,
  since the menu config names them; a layout and its `.meta` are committed together.

## Related documentation

- [Mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
  — the screen as built, its
  data, design target, open work and decisions
- [Mission selection design references](/documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/README.md)
  — the Stitch mockup sets and the Arma 3 captures the screen started from
