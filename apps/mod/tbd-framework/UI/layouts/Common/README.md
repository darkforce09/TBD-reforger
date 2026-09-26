# Shared interface layouts

The framework's component library: the panels, rows, chips, inputs, grids and rounded shapes that
every pre-game screen and the shell-based menus are assembled from. Each layout pairs with a
handler class in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/` or
`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/` that binds its widgets by name.

## Contents

```text
apps/mod/tbd-framework/UI/layouts/Common/
├── TBD_Button.layout*        runtime button: bottom-bar actions, Locate and Load Plan buttons
├── TBD_Caption.layout*       uppercase mono section label with a dim trailing note
├── TBD_Chip.layout*          tinted mono pill for badges, tags and counts
├── TBD_Columns2.layout*      two equal columns, `ColumnA` and `ColumnB`
├── TBD_Columns3.layout*      three equal columns for the kit and asset grids
├── TBD_Columns4.layout*      four equal columns for the kit and asset grids
├── TBD_Dropdown.layout*      popover trigger: label, badge dock and chevron
├── TBD_DropdownMenu.layout*  the popover a dropdown mounts into a screen's `OverlayDock`
├── TBD_InsetText.layout*     wrapped paragraph in an inset box
├── TBD_KeyValueRow.layout*   key on the left, mono value on the right, optional chips
├── TBD_ListRow.layout*       one pooled row of a `TBD_ListBox`: accent, title, detail
├── TBD_NavItem.layout*       one tab of a tab strip: icon, label, badge, optional rule above
├── TBD_NumberedCard.layout*  numbered card with title, chip, paragraph, body and footer docks
├── TBD_Panel.layout*         glass card sized to its content, stacked in a scrolling list
├── TBD_PanelFill.layout*     the same card anchored to fill its dock, for columns and pages
├── TBD_Rounded*.layout*      rounded-rectangle shapes, one file per radius from 5 to 12
├── TBD_ScreenShell.layout*   the full-screen shell of the list-based menus: admin, spectator
├── TBD_ScrollBar.layout*     the 4 px track and thumb that replaces the engine scroll bar
├── TBD_ScrollList.layout*    a clipped scrolling list with its scroll-bar dock
├── TBD_SearchBox.layout*     search field with icon and clear button
├── TBD_Section.layout*       collapsible card whose header button folds the body
├── TBD_StatCell.layout*      label, value and count cell of the kit and asset grids
└── TBD_TabStrip.layout*      segmented control that instantiates a nav item per entry
```

Each `.layout` sits beside its `.layout.meta`, so every line covers the pair.

## How it works

A screen never names a file here by path: `TBD_UILayouts`
(`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`) holds one constant per layout,
and `TBD_UILayouts.Create`, `CreateStretched` or `CreateHandler` instantiates it under a parent.
Most primitives carry their handler as a widget component on the root, and several handlers add a
static `Mount(dock, …)` that creates and binds in one call. The handler finds each named widget
with `FindAnyWidget` and every write is null-safe, so a widget missing from a copy of a layout is
skipped.

| Layout | Handler | Block | Widgets the handler binds | Mounted by |
|---|---|---|---|---|
| `TBD_ScreenShell` | `TBD_ShellScreen` menu | `07` | `Backdrop`, `Panel`, `Title`, `Subtitle`, `BackAction`, `List`, `Status`, `PrimaryAction` | the `TBD_UIShell`, `TBD_Spectator` and `TBD_UIAdmin` presets |
| `TBD_ListRow` | `TBD_ListBoxRow` | `07` | `Background`, `Accent`, `Title`, `Detail` | `TBD_ListBox`, the default of its row-layout attribute |
| `TBD_Panel`, `TBD_PanelFill` | `TBD_PanelComponent` | `10`, `25` | `PanelBorder`, `PanelBG`, `HeaderRow`, `HeaderBG`, `HeaderIcon`, `HeaderTitle`, `HeaderBadgeDock`, `HeaderRule`, `BodyDock`, `FooterDock` | Panel: [mission](/documentation_v2/glossary/g_to_m.md#mission) and kit inspector cards; PanelFill: selector, lobby and briefing columns and pages |
| `TBD_Chip` | `TBD_ChipComponent` | `11` | `ChipBorder`, `ChipBG`, `ChipDot`, `ChipText` | every badge, tag and count |
| `TBD_SearchBox` | `TBD_SearchBoxComponent` | `12` | `SearchBorder`, `SearchBG`, `SearchIcon`, `SearchInput`, `SearchClear`, `SearchClearGlyph` | terrain and mission search |
| `TBD_NavItem` | `TBD_NavItemComponent` | `13` | `Border`, `Background`, `Icon`, `Label`, `Badge`, `SeparatorSize`, `Separator` | `TBD_TabStripComponent` only |
| `TBD_TabStrip` | `TBD_TabStripComponent` | `13` | `StripBorder`, `StripBG`, `ItemsRow`, `ItemsColumn` | the session top bar's tabs |
| `TBD_Button` | `TBD_UIButton` | `14` | `Border`, `Background`, `Label` | bottom-bar actions, Locate buttons, Load Plan |
| `TBD_KeyValueRow` | `TBD_KeyValueRowComponent` | `16` | `RowBorder`, `RowBG`, `KeyIcon`, `KeyChipDock`, `KeyText`, `ValueText`, `ValueChipDock` | mission and kit inspectors, briefing pages |
| `TBD_Dropdown` | `TBD_DropdownComponent` | `17` | `TriggerBorder`, `TriggerBG`, `TriggerLabel`, `TriggerBadgeDock`, `Chevron` | the Modes filter, the version picker, the markers plan |
| `TBD_DropdownMenu` | `TBD_DropdownComponent` | `17` | `Scrim`, `Menu`, `MenuBorder`, `MenuBG`, `MenuTitle`, `SelectAll`, `DeselectAll`, `MenuRule`, `MenuList` | a dropdown, into the screen's `OverlayDock` |
| `TBD_InsetText` | none; the screen writes `Body` | `18` | `InsetBorder`, `InsetBG`, `Body` | mission summary, briefing background page |
| `TBD_Columns2`, `3`, `4` | none | `24`, `38`, `39` | `ColumnA` to `ColumnD` | inspector, kit and asset grids |
| `TBD_ScrollBar` | `TBD_UIScrollBar` | `2E` | `Track`, `Thumb` | every list's `ScrollBarDock` |
| `TBD_StatCell` | none; the owner paints it | `36` | `Border`, `Background`, `Label`, `Value`, `Count` | kit inspector and briefing grids |
| `TBD_Section` | `TBD_SectionComponent` | `3A` | `Border`, `Background`, `HeaderButton`, `HeaderOverlay`, `HeaderBG`, `HeaderIcon`, `Title`, `BadgeDock`, `ActionDock`, `Chevron`, `HeaderRule`, `Body` | briefing assets and rules pages |
| `TBD_NumberedCard` | `TBD_NumberedCardComponent` | `3B` | `Border`, `Background`, `NumberPill`, `Number`, `Title`, `ChipDock`, `Body`, `BodyDock`, `FooterRule`, `FooterDock` | briefing objectives and rules |
| `TBD_Caption` | `TBD_Caption.Mount` | `3C` | `Caption`, `Trailing` | briefing pages, markers panel |
| `TBD_ScrollList` | `TBD_ScrollList.Mount` | `3D` | `ListFrame`, `Scroll`, `Content`, `ScrollBarDock` | briefing page bodies, player lanes |
| `TBD_Rounded5` to `12` | `TBD_UILayouts.MountRounded` | `26` to `2D` | `Centre`, `Left`, `Right`, `CornerTL`, `CornerTR`, `CornerBL`, `CornerBR` | every `*Border` and `*BG` frame dock |

### Engine limits the library works around

- **No corner radius or 9-slice.** Each `TBD_Rounded*` shape is seven images: a centre, two side
  strips, and four clipping frames that each show a quarter of the white disc
  `apps/mod/tbd-framework/UI/Textures/TBD/TBD_Disc_UI.edds`. A handler calls
  `TBD_UILayouts.MountRounded(dock, radius)` on an empty `FrameWidgetClass` dock (`*Border`,
  `*BG`, `Background`): the border takes the even radius and the 1 px-inset fill the odd radius
  below it, so the arcs are concentric. A dock left as an image widget stays square. The radii are
  `TBD_UITheme.RADIUS_PANEL` 12, `RADIUS_ROW` 8, `RADIUS_TAG` 6 and `RADIUS_PILL` 10; to square
  only the lower corners, a header band's fill is taller than its clipping parent.
- **No runtime font setter.** A `TextWidget` cannot change font from script, so each text widget
  carries a `FontProperties` block naming `Roboto_Bold` (headers, titles, labels),
  `RobotoCondensed_Regular` (body and key text) or `robotomono_msdf_28` (chips, counts, values,
  search input); `TBD_UITheme` lists the GUIDs as `FONT_*`. `TBD_ScreenShell` and `TBD_ListRow`
  carry none and draw in the engine's default font.
- **An unstyleable engine scroll bar.** A `ScrollLayoutWidget` draws its own bar over the right
  edge. `TBD_ScrollList` holds the recipe every list layout repeats: the frame around `Scroll`
  clips, the `Scroll` slot overhangs the right edge by 24 px, `Content` pads 34 px on the right
  (24 hidden plus a 10 px gutter), and a 4 px `ScrollBarDock` takes a `TBD_UIScrollBar`.
- **Linear blending.** Colours in these files are placeholders: the handler paints every surface
  from a `TBD_UITheme` token or `TBD_EUITint`, flattened to an opaque colour over the ground it
  sits on, and paints each image of a rounded shape itself because a frame's colour does not reach
  its children.

## Format

- File type: [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) widget layouts (`.layout`), plain
  text: a tree of `<Type>WidgetClass "{GUID}" { Name "…" Slot … components { … } { children } }`.
  The `.layout.meta` beside each is a `MetaFileClass` whose `Name` holds
  `{GUID}UI/layouts/Common/<file>.layout` and whose `LayoutResourceClass` entries cover each
  platform.
- Resource GUID: `7BD1A7000000XXnn`, where `XX` is the layout's block and `nn` numbers the widgets;
  the `.meta` takes `nn` = `01`, and a second layout sharing a block takes the next free number
  (`TBD_ListRow` `0702`, `TBD_TabStrip` `1302`, `TBD_DropdownMenu` `1702`). The block ledger is the
  header of `TBD_UILayouts`; a GUID never changes once a constant or config names it.
- Naming: `TBD_<Component>.layout`; the radius of a shape is its file name.
- Adding a primitive: take a free block from the ledger, author the layout with `FrameWidgetClass`
  docks for its rounded surfaces and a `FontProperties` block on each text, write the `.meta`, add
  the constant to `TBD_UILayouts`, and commit both files;
  [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) must rewrite
  `resourceDatabase.rdb` before the game can find the new path.

## Referenced by

- `TBD_UILayouts` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`, by GUID and
  path, one constant per layout; the handlers and screens under
  `apps/mod/tbd-framework/Scripts/Game/TBD/` use those constants.
- `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`, by GUID: the `TBD_UIShell`,
  `TBD_Spectator` and `TBD_UIAdmin` menu presets open `TBD_ScreenShell.layout`.
- `TBD_ListBox` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_ListBox.c`, by GUID: its
  row-layout attribute defaults to `TBD_ListRow.layout`.
- The layouts in `apps/mod/tbd-framework/UI/layouts/Session/` and `Hud/` name the handler classes
  and repeat the dock and scroll conventions; they do not include these files.

## Boundaries

- Depends on: the texture `apps/mod/tbd-framework/UI/Textures/TBD/TBD_Disc_UI.edds`; the game's
  Roboto fonts; the handler classes in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/` and
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`.
- Used by: the Mission Selector, lobby, briefing and players screens in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/`, the session bars, and the admin and spectator
  menus through the shell presets.
- Rules: a widget name a handler binds is part of its contract and changes only with the handler;
  every layout has a `TBD_UILayouts` constant and nothing instantiates a bare path; a block is
  taken from the ledger after a `git grep` for it; the layout and its `.meta` are committed
  together; `cargo xtask mod compile` checks the handler scripts.

## Related documentation

- [Mod UI structure](/documentation_v2/mod/tbd-framework/UI/README.md) — where each UI file goes
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md)
  — the design rules the theme and shapes encode
