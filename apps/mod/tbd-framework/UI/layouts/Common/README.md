| `TBD_ScrollBar.layout` | none (`TBD_UIScrollBar.Mount` in Core) | `2E` | `Track`, `Thumb` (FrameSlot; script sets pos + height) | `Mount(dock, scroll, content, ground)`, `SetGround`, `Destroy` — ticks at 30 Hz, hides when nothing scrolls | every list: terrain rows, mission cards, inspector stack, dropdown menus |
# UI/layouts/Common — the component library

Atomic primitives every TBD screen is assembled from. Each row below is one `.layout` (plus its
`.layout.meta`), one handler class in `Scripts/Game/TBD/UI/Core/` or `UI/Common/`, and a widget-name
contract the handler binds by `FindAnyWidget`. Every write in every handler is null-safe, so a
screen may ship a stripped copy of a layout and lose only the widgets it dropped.

Read this table before authoring a new panel: if the shape is here, mount it; do not redraw it.

| Layout | Handler | GUID block | Widget contract (names) | API | Used by |
|---|---|---|---|---|---|
| `TBD_ScreenShell.layout` | `TBD_ShellScreen` (Core) | `07` | `Backdrop`, `Panel`, `Title`, `Subtitle`, `BackAction`, `List`, `Status`, `PrimaryAction` | `SetTitle`, `SetSubtitle`, `SetStatus`, `SetPrimaryAction`, `GetList` | Spectator, Admin, bare `TBD_UIShell` |
| `TBD_ListRow.layout` | `TBD_ListBoxRow` (Core) | `07` (`0750+`) | `Background`, `Accent`, `Title`, `Detail` | pooled by `TBD_ListBox` | every `TBD_ListBox`, dropdown menus |
| `TBD_Panel.layout` | `TBD_PanelComponent` | `10` | `PanelBorder`, `PanelBG`, `HeaderRow`, `HeaderBG`, `HeaderIcon`, `HeaderTitle`, `HeaderBadgeDock` (row), `HeaderRule`, `BodyDock`, `FooterDock` | `SetTitle` (shouts), `SetIcon`, `ShowHeader`, `SetTint`, `GetBadgeDock`, `GetBodyDock`, `GetFooterDock` | Mission Selector columns + inspector cards; every briefing / lobby panel to come |
| `TBD_PanelFill.layout` | `TBD_PanelComponent` | `25` | same names as `TBD_Panel` | same API; **frame-anchored** — fills its dock, body takes the remaining height. Columns use this; cards use `TBD_Panel` (a frame-anchored body inside `TBD_Panel` collapses to zero height) | selector TERRAINS + MISSIONS columns |
| `TBD_Chip.layout` | `TBD_ChipComponent` | `11` | `ChipBorder`, `ChipBG`, `ChipDot`, `ChipText` | `Set(text, tint)`, `SetText`, `SetTint`, `SetDotVisible`, static `Mount(dock, text, tint)` | every badge / tag / count pill |
| `TBD_SearchBox.layout` | `TBD_SearchBoxComponent` | `12` | `SearchBorder`, `SearchBG`, `SearchIcon`, `SearchInput` (EditBox), `SearchClear`, `SearchClearGlyph` | `GetQuery`, `Clear`, `SetPlaceholder`, `GetOnChanged()(box, query)`, static `Matches(query, text)` | terrain + scenario search; lobby roster next |
| `TBD_NavItem.layout` | `TBD_NavItemComponent` | `13` (`1300`, children `1303+`) | `Border`, `Background`, `Icon`, `Label`, `Badge` | `Bind(strip, index, TBD_NavItemData)`, `SetActive` | instantiated by `TBD_TabStrip` only |
| `TBD_TabStrip.layout` | `TBD_TabStripComponent` | `13` (`1340+`, meta `1302`) | `StripBorder`, `StripBG`, `ItemsRow`, `ItemsColumn` | `SetItems(array<ref TBD_NavItemData>)`, `SetActive`, `GetOnSelected()(strip, index)`, `FocusActive`; attrs `m_bVertical`, `m_bChrome` | top bar (row); briefing primary + topic nav (column) next |
| `TBD_Button.layout` | `TBD_UIButton` (Core) | `14` | `Border`, `Background`, `Label` | `SetLabel`, `SetPrimary`, `SetTint` (PRIMARY / SUCCESS / WARNING for toggled states), `SetInteractive`, `GetOnActivate()` | `TBD_SessionBottomBar.AddAction` |
| `TBD_KeyValueRow.layout` | `TBD_KeyValueRowComponent` | `16` | `RowBorder`, `RowBG`, `KeyIcon`, `KeyChipDock`, `KeyText`, `ValueText`, `ValueChipDock` | `Set(key, value, valueTint)`, `SetKeyChip`, `SetValueChip`, `SetIcon`, `SetTint`, static `Mount(container)` | inspector ORBAT / objective rows; parameters, frequencies, kit lines next |
| `TBD_Dropdown.layout` | `TBD_DropdownComponent` | `17` (`1700`, children `1703+`) | `TriggerBorder`, `TriggerBG`, `TriggerLabel`, `TriggerBadgeDock`, `Chevron` | `SetItems(array<ref TBD_DropdownItem>)`, `SetMultiSelect`, `SetMenuTitle`, `SetOverlayHost`, `GetSelectedTag`, `IsChecked`, `GetCheckedCount`, `GetOnChanged()(dropdown, tag)`, static `Mount(dock, overlayHost, label, multi)` | Modes filter (multi), version picker (single); markers plan next |
| `TBD_DropdownMenu.layout` | created by `TBD_DropdownComponent` | `17` (`1740+`, meta `1702`) | `Scrim`, `Menu`, `MenuBorder`, `MenuBG`, `MenuTitle`, `SelectAll`, `DeselectAll`, `MenuRule`, `MenuList` (`TBD_ListBox`) | mounted into the screen's `OverlayDock`, positioned with `FrameSlot` | — |
| `TBD_InsetText.layout` | none (screen writes `Body`) | `18` | `InsetBorder`, `InsetBG`, `Body` (wrapping text) | — | mission summary; lore, rules next |
| `TBD_Columns2.layout` | none | `24` | `ColumnA`, `ColumnB` (vertical layouts, equal fill) | — | mod grid, ORBAT + objective faction pairs |
| `TBD_Columns3.layout`, `TBD_Columns4.layout` | none | `38`, `39` | `ColumnA..C` / `ColumnA..D` (equal fill, 6 px gutters) | — | kit inspector grids (weapons ×3; gear / gadgets / tools / misc ×4) |
| `TBD_Rounded5…12.layout` (8 files) | none (`TBD_UILayouts.MountRounded`) | `26-2D` | `Centre`, `Left`, `Right`, `CornerTL/TR/BL/BR` (clipping frames) each holding `Disc*` (our `UI/Textures/TBD/TBD_Disc_UI.edds` filled disc, GUID `{1F2DC726318EC5AF}`; vanilla `circleFull.edds` is a ring); painted by `TBD_UITheme.PaintOver` on the dock (it walks the `Rounded*` subtree — `"Inherit Color"` does NOT propagate from a frame) | mounted into a `*Border` / `*BG` frame dock; radius is the file | every rounded surface below |

## Rules that make the table true

- **Colour is a tint, never a literal.** `TBD_UITheme.ChipFill/ChipBorder/ChipInk` and
  `PanelFill/PanelBorder` take a `TBD_EUITint` (`NEUTRAL PRIMARY SUCCESS WARNING DANGER TERTIARY
  BLUFOR OPFOR SOLID`). A new hue is a theme edit, not a handler edit.
- **Colour is composited in sRGB, by us.** The browser blends `rgba()` in sRGB, Enfusion in
  linear, so a translucent token must never reach the engine: `TBD_UITheme.Paint` flattens it
  with `Over(token, ground)` first. Every chrome handler has `SetGround(opaque)` and a panel
  exposes `GetGround()` (its composited fill); whoever mounts a chip, row, search box or dropdown
  inside a panel passes `panel.GetGround()`. Defaults: a panel assumes the backdrop, everything
  else assumes a glass panel. `PaintAlpha` is the only real-alpha path (SCRIM over the world).
- **Curves.** Enfusion has no corner radius and no 9-slice. Every `*Border` / `*BG` in the table
  is an empty `FrameWidgetClass` dock; the handler calls `TBD_UILayouts.MountRounded(dock, r)`
  once at attach — border at `r`, the 1 px-inset fill at `r - 1` so the arcs are concentric — and keeps painting the dock (`PaintOver` tints the seven images itself; a transparent paint hides the shape). Radii come from
  `TBD_UITheme`: `RADIUS_PANEL` 12 (panels, inspector, dropdown menu), `RADIUS_ROW` 8 (rows, cards,
  inputs, buttons, nav items, key-value rows, mod items, inset text), `RADIUS_TAG` 6 (chips),
  `RADIUS_PILL` 10 (`TBD_ChipComponent.SetPill(true)`: count pills). A dock left as an
  `ImageWidgetClass` is ignored by `MountRounded` and stays square — that is the opt-out.
- **Fonts are explicit.** Every `TextWidgetClass` / `EditBoxWidgetClass` here carries a `FontProperties FontProperties "{GUID}" { Font "…" }` block (a bare `Font` line is an `Unknown keyword`):
  `Roboto_Bold` for uppercase headers, card titles, nav + button labels; `RobotoCondensed_Regular`
  for body and key text (`_Bold` for the selected row title / faction role); `robotomono_msdf_28`
  for chips, counts, versions, values and search input. GUIDs and the size ladder
  (`TEXT_HEADER 14 · TEXT_BODY 13 · TEXT_MONO_SM 12 · TEXT_TAG 11`) live in `TBD_UITheme`; the
  engine has no runtime font setter, so a font change is a layout edit.
- **Engine scrollbars are clipped away.** `ScrollLayoutWidget` draws its own ~10 px white bar over
  the right edge of the content and there is no property to style it (vanilla hides it the same
  way: `SCR_PooledListComponent.ShowScrollbar`, "clip scroll bar to hide"). Recipe, used by every
  list layout: the frame around `Scroll` gets `Clipping True`; the `Scroll` slot overhangs the
  right edge by 24 (`SizeX +24`, `OffsetRight -24`); `Content` gets `Padding 0 0 34 0` (24 hidden
  + 10 gutter); a 4 px `ScrollBarDock` frame sits on the frame's right edge and the owner mounts
  `TBD_UIScrollBar` into it. Rows end at the gutter with their right corners visible.
- **Round only some corners = clip the shape.** A `Rounded*` shape has four round corners; to
  square the bottom ones (panel header band) the dock is made taller than its clipping parent
  (`HeaderRow`/`HeaderOverlay` clip; `HeaderBG` extends 12 px below) so the bottom arcs fall
  outside the clip. No extra layout files.
- **A photo under round corners = inverse-disc masks.** Nothing clips an image to an arc, so the
  inspector hero paints clipped quarters of `TBD_DiscInv_UI` over its top corners: inside the hero,
  r11 quarters in the panel BORDER colour; on the panel root, r12 quarters in the BACKDROP colour.
  Photo inside r11, border ring r11–r12, backdrop beyond. `TBD_MissionInspectorPanel.MountCornerMask`.
- **Textures are ours.** Vanilla texture GUIDs are pak-only, so every image the framework needs is
  a committed PNG under `UI/Textures/TBD/` that the operator imports in Workbench (`TextureUI.conf`,
  as the disc was); the import writes the `.edds` + `.meta`, and the `.meta` GUID is then pinned
  into `TBD_UILayouts` (all five current textures are pinned; a new PNG is a bare path until its import). `TBD_UILayouts.LoadTexture` hides the widget and
  logs once when a texture is missing, so a not-yet-imported PNG costs a hidden slot, not a white
  quad. Sources: disc / inverse disc / fade / topo art from a scratch Node rasteriser; the Everon
  hero is `ffmpeg -i packages/map-assets/everon/tiles/satellite/full.webp -vf "crop=4096:560:0:1600,scale=1024:140"`.
- **Icons are keys.** Image slots are fed by `TBD_UIIcons.Load(widget, key)`; an unresolved key
  hides the slot and logs once. See `Scripts/Game/TBD/UI/Core/README.md` for the honest status of
  the quad table.
- **Shrink-wrap vs stretch.** Chips, buttons, nav items and dropdown triggers declare an
  `AlignableSlot` root and size to their text. Panels, search boxes and rows declare a stretched
  root; when mounted into a layout widget the mounting code calls
  `AlignableSlot.SetHorizontalAlign(..., Stretch)` (`TBD_UILayouts.CreateStretched`) because a
  root slot only exists after the widget has a parent.
- **Docks.** A widget named `*Dock` is an empty container the owner mounts into. Overlay docks
  shrink-wrap one child; `HeaderBadgeDock` is a horizontal row so two chips can sit side by side.
- **GUIDs** are `7BD1A7000000XXnn`; the block ledger lives in the header of
  `Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`. Grep before taking a block.
- **Registration.** Every layout here has a constant in `TBD_UILayouts`; nothing instantiates a
  bare path. New files are invisible until Workbench rewrites `resourceDatabase.rdb`.
