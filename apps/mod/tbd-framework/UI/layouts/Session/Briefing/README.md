# UI/layouts/Session/Briefing — the Briefing screen (rebuilt 2026-09-14)

Dock & Sub-Layout screen over the live map. The shell holds the full-screen `SCR_MapEntity`
(`MapFrame`, vanilla `Map.layout` include) and empty docks; `Session/Briefing/UI/TBD_BriefingScreen.c`
mounts everything at open. **The map never resizes** — every panel is an overlay, and the shell has
no `Backdrop`.

```
 MapFrame
 ┌ TopDock 56 ─ Session/Shared/TBD_SessionTopBar ─────────────────────────────────────┐
 │ LeftDock 288        CenterDock 340 (x 300)     RightDock (x 652, width per page)    │
 │ primary nav         topic nav                  page / markers panel                 │
 └ BottomDock 64 ─ Session/Shared/TBD_SessionBottomBar ───────────────────────────────┘
   OverlayDock — dropdown menus
```

Page widths (`TBD_BriefingNav.PageWidth`, applied as `PageHost.SetWidthOverride` per page — `FrameSlot.SetSize` on the dock did not resize it, MEASURED run 3): frequencies 448 · ORBAT 1200 ·
assets 576 · uniforms 768 · objectives 440 · rules 480 · background 440 · parameters 440. The Markers panel takes the topic nav's column (CenterDock, 340) instead of the page column.

| File | Block | Contract | Driven by |
|---|---|---|---|
| `TBD_BriefingScreen.layout` | `0D` (meta `0D01`, `MapFrame` `0D02`, `ToolMenu` `0DF0+`) | `MapFrame`, `ToolMenu` (vanilla `SCR_MapToolMenuUI` binds it by name — top-right under the top bar), `WindowFrame`, `TopDock`, `LeftDock`, `CenterDock`, `RightDock` (runs to the screen edge) → `PageHost` (a `SizeLayoutWidget`; its `SetWidthOverride` IS the page width) → `PageFrame` (the FrameWidget pages mount into), `WideDock` (x 300, 800 wide; PLAYERS mode), `BottomDock`, `OverlayDock` | `TBD_BriefingScreen` |
| `TBD_FreqRow.layout` | `3E` | `Border`, `Background`, `Name`, `FreqChipDock`, `AuxRule`, `AuxLabel`, `AuxValue` | `TBD_BriefingFrequenciesPage` (LR net = gold chip, own squad net = primary chip + selected border) |
| `TBD_OrbatPage.layout` | `3F` | `RosterDock` (500), `KitDock` (rest) | `TBD_BriefingOrbatPage` — mounts the lobby's `PANEL_FILL` + `TBD_LobbyRosterPanel` (read-only) and `LOBBY_KIT_INSPECTOR` + `TBD_KitInspectorPanel` |
| `TBD_AssetPreview.layout` | `40` | `PreviewBorder`, `PreviewBG`, `GridClip`/`GridImage`/`Preview` (ItemPreviewWidget, 168 tall), `Label` | `TBD_BriefingAssetsPage` — the Vehicle Info box, `TBD_KitPreviewComponent.ShowVehicle(prefab)` |
| `TBD_UniformCard.layout` | `41` | `Border`, `Background`, `Name`, `HeaderRule`, `PreviewBorder`/`PreviewBG`/`GridImage`/`Preview` (240 tall)/`Label`, `ChipsDock`, `CamoLabel` | `TBD_BriefingUniformsPage` — the doll is the faction rifleman prefab, `ShowPrefab(prefab, null, name)` |
| `TBD_MarkersPanel.layout` | `42` | `PanelBorder`, `PanelBG`, `CaptionDock`, `PlanDropdownDock`, `LoadButtonDock` | `TBD_BriefingMarkersPanel` — plan `TBD_Dropdown` + `Load Plan` (logs only; no plan store yet) |
| `TBD_PrimaryNav.layout` | `46` | `PanelBorder`, `PanelBG`, `Items` (glass panel, sized to its items) | `TBD_BriefingPrimaryNav` |
| `TBD_PrimaryNavItem.layout` | `47` | `Border`, `Background`, `AccentSize`/`Accent` (right bar, active only), `IconBoxBorder`/`IconBoxBG`/`Icon` (32 px box), `Label`, `BadgeDock` | `TBD_PrimaryNavItemComponent` — active = `NAV_ITEM_ACTIVE_*` fill + accent, hover = white/5, Players carries BLUFOR / OPFOR count chips |
| `TBD_TopicNav.layout` | `49` | `PanelBorder`, `PanelBG`, `Items` (340 wide glass panel, sized to its items, right of the primary nav) | `TBD_BriefingTopicNav` |
| `TBD_TopicNavItem.layout` | `48` | `SeparatorSize`/`Separator` (group rule), `Border`, `Background`, `Icon`, `Label` | `TBD_TopicNavItemComponent` — active = solid `TOPIC_ITEM_ACTIVE_FILL`, hover = slate-800/50 + faint border |

Both navs are their own panels (`TBD_BriefingPrimaryNav`, `TBD_BriefingTopicNav`); everything else on the pages is Common: `TBD_PanelFill` (page chrome), `TBD_ScrollList` (body),
`TBD_Section` (rules groups, asset types, Vehicle Info, asset instances), `TBD_NumberedCard`
(objectives, rules), `TBD_KeyValueRow` (parameters, specs, ammunition), `TBD_Caption`, `TBD_StatCell`
grids in `TBD_Columns2/3`, `TBD_InsetText` (background). The PLAYERS modal is
`Session/Shared/TBD_PlayersPanel.layout`, mounted into `WideDock` (x 300 → right edge) in PLAYERS mode.

Input: `MapContext` is armed only while the cursor is off the chrome (`TBD_BriefingScreen.CursorOverChrome`), so the wheel scrolls a panel instead of zooming the map; the preset declares no ActionContext.

Rules: every `*Border` / `*BG` / `Background` is a `FrameWidgetClass` dock → `MountRounded`
(panels 12, rows / cards 8, preview boxes 6); every text carries `FontProperties`; lists use the
scroll-clip recipe through `TBD_ScrollList`; enemy pages are the friendly builders painted
`TBD_EUITint.OPFOR` (header chip, count chip, section borders), without Locate.
