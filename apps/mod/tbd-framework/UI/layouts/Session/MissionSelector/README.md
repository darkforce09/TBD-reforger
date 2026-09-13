# UI/layouts/Session/MissionSelector — the Scenario Browser screen

First Dock & Sub-Layout screen (2026-09-12). The 2736-line monolith is gone; the shell below is
under 130 lines and everything visible is mounted into it at runtime by
`Scripts/Game/TBD/Session/MissionSelector/UI/TBD_MissionSelectorScreen.c`.

## Shell — `TBD_MissionSelector.layout` (GUID `{7BD1A70000000B01}`, block `0B`)

Same path and GUID as the retired monolith, so `Configs/System/chimeraMenus.conf` and the rdb row
did not move. Backdrop + `WindowFrame` (16 px margin) + docks:

| Dock | Geometry (reference px) | Mounted layout | Driven by |
|---|---|---|---|
| `TopDock` | full width, 56 high | `Session/Shared/TBD_SessionTopBar` | `TBD_DockScreen` |
| `LeftDock` | x 0, width 320, y 68 → bottom-76 | `Common/TBD_PanelFill` + `TBD_TerrainSelector` body | `TBD_TerrainSelectorPanel` |
| `CenterDock` | x 332, width 440 | `Common/TBD_PanelFill` + `TBD_ScenarioBrowser` body | `TBD_ScenarioBrowserPanel` |
| `RightDock` | x 784 → right edge | `TBD_MissionInspector` | `TBD_MissionInspectorPanel` |
| `BottomDock` | full width, 64 high | `Session/Shared/TBD_SessionBottomBar` | `TBD_DockScreen` |
| `OverlayDock` | full-bleed, last child, hidden while empty | dropdown menus | `TBD_DropdownComponent` |

## Sub-layouts

| Layout | GUID block | Role | Widget contract |
|---|---|---|---|
| `TBD_TerrainSelector.layout` | `1D` | body of the TERRAINS panel | `SearchStrip`, `SearchDock`, `SearchRule`, `ListFrame` (clips), `Scroll`, `Content`, `ScrollBarDock`, `EmptyState` |
| `TBD_TerrainRow.layout` | `22` | one pooled terrain row (`TBD_TerrainRowComponent`) | `Border`, `Background`, `Accent` (glow bar), `Icon`, `Title`, `CountBadgeDock`, `Chevron` |
| `TBD_ScenarioBrowser.layout` | `1E` | body of the <TERRAIN> MISSIONS panel | `ToolRow`, `SearchDock`, `ModesDock`, `ToolRule`, `ListFrame` (clips), `Scroll`, `Content`, `ScrollBarDock`, `EmptyState` |
| `TBD_MissionCard.layout` | `23` | one pooled mission card (`TBD_MissionCardComponent`) | `Border`, `Background`, `TagChipDock`, `TerrainText`, `SlotCount`, `PulseDot`, `Title`, `Indicator`, `IndicatorGlyph` |
| `TBD_MissionInspector.layout` | `1F` | right column: photo hero + scrolling card stack | `PanelBorder`, `PanelBG`, `Hero` (clips; `HeroImage`, `HeroDim`, `HeroFade`, `HeroMaskTL/TR` + `*Img`, `TitleRow`: `HeroTitle`, `HeroTagDock`; `AuthorRow`: `AuthorIcon`, `AuthorBy`, `AuthorName`, `AuthorChipDock`; `VersionDock`), `PanelMaskTL/TR` + `*Img`, `BodyFrame` (clips), `Scroll`, `CardsContent`, `ScrollBarDock`, `EmptyState` |
| `TBD_ModGridItem.layout` | `20` | one mod in the REQUIRED MODSET grid | `Border`, `Background`, `CheckIcon`, `CheckGlyph`, `ModName`, `VersionChipDock` |
| `TBD_FactionColumn.layout` | `21` | body of a faction-tinted `TBD_Panel` (ORBAT / objectives) | `ColumnHeader` (`FactionChipDock`, `RoleText`, `CountText`, `CountLabel`), `HeaderRule`, `Rows` |

The inspector's four cards are `Common/TBD_Panel` instances mounted into `CardsContent`; their
bodies are `Common/TBD_Columns2`, `Common/TBD_InsetText`, `Common/TBD_KeyValueRow` and the two
selector-specific items above.

## Curves and grounds (pass 3)

| Surface | Radius | Ground it composites over |
|---|---|---|
| TERRAINS / MISSIONS columns (`PanelFill`), inspector panel, top + bottom bars | `RADIUS_PANEL` 12 | backdrop (`Ground()`) |
| top-bar identity + count boxes | `RADIUS_ROW` 8 | the bar fill |
| terrain rows, mission cards, search boxes, Modes / version triggers, mod items, summary inset, key-value rows | `RADIUS_ROW` 8 | the owning panel's `GetGround()`; faction rows over the faction column's fill |
| tag chips (`PVP`, `COOP`, `AUTHOR`, `v2.4.0`, `2x`) | `RADIUS_TAG` 6 | the row / card / band they sit on (passed by the mounter) |
| `N AVAILABLE`, Modes count | `RADIUS_PILL` 10 (`SetPill`) | panel header / trigger fill |
| inspector cards (`TBD_Panel`) | 12 | `PanelGround()` (set in `MountCard`) |
| faction columns (tinted `TBD_Panel`) | 12 | the ORBAT / objectives card fill |

Square on purpose: rules, the row `Accent` glow bar, the bottom edge of every panel header band (its `HeaderBG` is clipped), the 4 px scrollbar.

## Hero (pass 4)

The inspector hero is a 176 px photo band: `HeroImage` (per terrain — `TBD_TerrainInfo.m_sHeroImage`;
Everon = `TBD_UILayouts.HERO_EVERON`, the satellite crop; the others = `HERO_TOPO`, the Stitch
topo-grid art painted at 25 %), `HeroDim` (`HERO_DIM`, real alpha), `HeroFade` (`TBD_FadeDown_UI`
tinted `HERO_FADE`, bottom 110 px), then the title / author / version rows sitting on the solid part
of the fade (`m_iHeroGround = HERO_FADE`). Its top corners are rounded by inverse-disc masks (see
`Common/README.md`, "A photo under round corners"). No rule under the hero: the first card starts
16 px below it. Everon band source: `packages/map-assets/everon/tiles/satellite/full.webp`,
`crop=4096:560:0:1600` → `1024x140` (move the crop by re-running the ffmpeg line in
`Common/README.md`).



## Data

Everything shown comes from `TBD_MissionCatalog.Get()`
(`Scripts/Game/TBD/Session/MissionSelector/TBD_MissionSelectorData.c`), filled today by
`Scripts/Game/TBD/UI/Mock/TBD_MissionSelectorMock.c`. Counts (`N AVAILABLE`, mode counts, slot
totals) are computed, never typed.

## Line budget

Largest file here is `TBD_MissionInspector.layout` at ~300 lines; the rule is 1000, the shell rule
is 200.
