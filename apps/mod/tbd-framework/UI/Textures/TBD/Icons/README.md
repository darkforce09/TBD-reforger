# Framework interface icons

The framework's own icon set: 38 white glyphs on a transparent ground that the pre-game screens
tint and show in their image slots. `TBD_UIIcons` turns an icon key into one of these textures, so a
screen names an icon by key and never by path.

## Contents

```text
apps/mod/tbd-framework/UI/Textures/TBD/Icons/
├── TBD_Icon_*_UI.edds       the imported texture the engine draws, one per icon key
├── TBD_Icon_*_UI.edds.meta  each texture's resource GUID and its TextureUI import settings
└── TBD_Icon_*_UI.png        the source raster of each icon, which Workbench imports
```

## How it works

A screen calls `TBD_UIIcons.Load(widget, key)`
(`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UIIcons.c`). When the key is one of the 38 in
`BuildShipped`, `Texture(key)` returns `UI/Textures/TBD/Icons/TBD_Icon_<key>_UI.edds` and `Load`
puts it on the image widget; the screen then paints the widget with a `TBD_UITheme` colour, which
is why every glyph is white. A key with no file here falls back to a quad of the vanilla
`icons_wrapperUI-64.imageset`, and a key with neither hides the slot. The keys are the Material
Symbols names the Stitch mockups use:

```text
ac_unit  adjust  assignment  cell_tower  check  checkroom  chevron_right  close  description
directions_car  download  edit_location_alt  expand_more  extension  flag  grid_view  groups
hourglass_bottom  landscape  lock  map  meeting_room  my_location  notes  person  play_arrow
radio  schedule  search  settings  shield  target  thermostat  timer  tune  visibility  warning
water
```

Every texture is addressed by its bare path: the pinned-GUID map `s_mTextureGuids` in
`TBD_UIIcons` is empty, so the `.meta` GUIDs are unused and the resource database resolves each
path.

## Format

- File type: each icon is three files. The `.png` is a 64 x 64 RGBA raster of the outlined Material
  Symbols glyph, white on alpha. The `.edds` is the texture
  [Workbench](/documentation_v2/glossary.md#workbench) writes when it imports the PNG, and the
  `.edds.meta` holds its resource GUID under `Name` and the per-platform
  `PNGResourceClass` settings, which inherit the vanilla `TextureUI.conf` for each platform.
- Resource GUID: the `.edds.meta` file; a GUID never changes once written.
- Naming: `TBD_Icon_<key>_UI`, where `<key>` is the Material Symbols name in snake_case.
- Adding an icon: the header of `TBD_UIIcons.c` holds the command that fetches the glyph's SVG and
  rasterises it to the PNG; import the PNG in Workbench with `TextureUI.conf`, add the key to
  `BuildShipped`, and commit the `.png`, `.edds` and `.edds.meta` together.

## Referenced by

- `TBD_UIIcons.Texture` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UIIcons.c`, by bare
  path built from the key; nothing refers to these textures by GUID.
- The icon keys come from the screens and their data: the briefing's navigation and pages in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`, the
  [mission](/documentation_v2/glossary.md#mission) inspector and mission browser in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/`, the kit inspector and
  roster in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/`, the players panel in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/UI/`, the search box and the session top
  bar in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/`, and the mock catalogs in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`.
- No script names the keys `check`, `close`, `download`, `expand_more`, `lock`, `my_location` or
  `play_arrow`, so those seven icons are unused.

## Boundaries

- Depends on: the vanilla `TextureUI.conf` import settings that each `.meta` inherits.
- Used by: `TBD_UIIcons` and, through it, every framework screen that shows an icon.
- Rules: the key list in `TBD_UIIcons.BuildShipped` matches the files here, since a key missing
  from the list never reaches its texture; the three files of an icon are committed together; the
  glyphs stay white, because the colour comes from the theme at run time; a new texture reaches a
  game build only after Workbench has imported it and rewritten `resourceDatabase.rdb`.

## Related documentation

- [Mod UI structure](/documentation_v2/mod/tbd-framework/UI/README.md)
  — where each UI file goes and the icon and colour rules
