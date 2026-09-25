# Framework interface assets

The framework's interface resources: the widget layouts every screen, menu and HUD element is
built from, and the textures they draw. The scripts that open and paint them live in
`apps/mod/tbd-framework/Scripts/Game/TBD/UI/` and in the `UI/` folder of each screen under
`apps/mod/tbd-framework/Scripts/Game/TBD/Session/`.

## Contents

```text
apps/mod/tbd-framework/UI/
├── layouts/   the widget layouts: shared primitives, the objective HUD and the session screens
└── Textures/  the framework's own textures: rounded-shape disc, hero art, masks and icons
```

## How it works

[Enfusion](/documentation_v2/glossary.md#enfusion) resolves a resource by the GUID in its `.meta`
file, or by its path inside the addon, through the addon's resource database. Screens never spell
a path: `TBD_UILayouts`
(`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`) holds one constant per layout
and per texture, `TBD_UIIcons` (same folder) maps icon keys to the icon textures, and
`apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` binds each menu preset to its shell
layout and screen class.

```text
chimeraMenus.conf ──preset──> layouts/Session/<screen>/<shell>.layout + screen class
TBD_UILayouts ──constant──> layouts/**.layout, Textures/TBD/*.edds
TBD_UIIcons ──key──> Textures/TBD/Icons/TBD_Icon_<key>_UI.edds
layouts/Common/TBD_Rounded*.layout ──GUID──> Textures/TBD/TBD_Disc_UI.edds
```

Four engine limits shape these files. Enfusion has no corner radius or 9-slice, so rounded
surfaces are layouts of seven images cut from one disc texture. A text widget cannot change font
from script, so each layout names its fonts. The engine blends colour in linear space, so a
layout's colours are placeholders that `TBD_UITheme` repaints with flattened, opaque tokens. The
vanilla texture GUIDs sit only inside the game's packed data, so every image the framework needs
is a committed PNG imported here.

## Format

- File type: widget layouts (`.layout`, plain text) and textures (a `.png` source with the `.edds`
  that the import writes); every resource has a `.meta` beside it whose `Name` holds
  `{GUID}UI/<path>`.
- Resource GUID: the `.meta` file. The layouts use `7BD1A7000000XXnn`, one block per layout from the
  ledger in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`; the textures keep the
  GUID the import wrote. A GUID never changes once a script constant or a config names it.
- Naming: `TBD_<Element>.layout` and `TBD_<Subject>_UI` textures; layouts sit in the folder of the
  screen that uses them, textures under `Textures/TBD/`.
- Adding or moving an asset: create or `git mv` it with its `.meta`, keep the `.meta` `Name` equal
  to the new path, update the `TBD_UILayouts` constant and, for a menu shell, the
  menu config's preset, then let [Workbench](/documentation_v2/glossary.md#workbench) open
  the addon and rewrite `apps/mod/tbd-framework/resourceDatabase.rdb`, since the game finds a
  non-script resource at a new path only through that database.

## Referenced by

- `TBD_UILayouts` and `TBD_UIIcons` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`, by GUID
  and path.
- `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`, by GUID: its six menu presets name
  the four shell layouts.
- The rounded-shape layouts, by GUID: the disc texture.

## Boundaries

- Depends on: the game's fonts, map layout, icon imageset and `TextureUI.conf` import settings;
  the script classes under `apps/mod/tbd-framework/Scripts/Game/TBD/` that the layouts name as
  components.
- Used by: the framework's screens, HUD and menus.
- Rules: every resource has one entry in `TBD_UILayouts` or `TBD_UIIcons`, and screens reach it only
  through them; a resource and its `.meta` are committed together; a new or moved resource needs a
  Workbench pass over `resourceDatabase.rdb`; `cargo xtask mod compile` checks the scripts that use
  them.

## Related documentation

- [Mod UI structure](/documentation_v2/mod/tbd-framework/UI/README.md)
  — where each UI file goes and which mockup panel lands in which folder
- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md)
  — the design rules the theme and layouts encode
- [Design tokens](/documentation_v2/design_system/design_tokens.md) — the website tokens
  `TBD_UITheme` mirrors, the mod's fonts, radii and spacing, and where the two differ
