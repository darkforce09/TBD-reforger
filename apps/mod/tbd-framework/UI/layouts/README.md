# Framework widget layouts

Every [Enfusion](/documentation_v2/glossary.md#enfusion) widget layout the framework's interface
draws: the shared primitives, the in-game HUD, and the session screens. Scripts instantiate them by
the resource names registered in `TBD_UILayouts`
(`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`).

## Contents

```text
apps/mod/tbd-framework/UI/layouts/
├── Common/   the component library: panels, rows, chips, inputs, grids, rounded shapes, list shell
├── Hud/      the objective panel shown over the world during live play
└── Session/  the pre-game screens, the post-game overlays and the bars and panels they share
```

## How it works

A layout is a tree of named widgets; the script class that opens it, a menu, a handler component
on the root, or a static `Mount` helper, finds its widgets by name and paints them. Screens are
built from small pieces: a shell of empty named docks, filled at run time with column bodies from
`Session/` and primitives from `Common/`, so each file stays small and a shape used twice exists
once. `TBD_UILayouts` holds the one constant per layout that every script uses, and its header
holds the ledger of GUID blocks.

```text
chimeraMenus.conf preset ──> screen shell (Session/…) ──> docks
                                                          ├─ Session/Shared bars
                                                          ├─ column bodies (Session/<screen>/)
                                                          └─ primitives (Common/)
stage change / RPC ──> workspace overlay (Hud/, Session/PostGame/)
```

## Format

- File type: Enfusion widget layouts (`.layout`), plain text, each beside a `.layout.meta` whose
  `Name` holds `{GUID}UI/layouts/<path>` and whose `LayoutResourceClass` entries cover each
  platform.
- Resource GUID: `7BD1A7000000XXnn`, where `XX` is the layout's block from the ledger in
  `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_UILayouts.c`, `nn` numbers its widgets and
  the `.meta` takes `01`. A GUID never changes once a script constant or a config names it.
- Naming: `TBD_<Element>.layout`, in the folder of the screen that uses it; shared primitives go in
  `Common/`.
- Adding a layout: take a free block after a `git grep` for it, author the layout and its `.meta`,
  add the `TBD_UILayouts` constant, and commit both files.
  [Workbench](/documentation_v2/glossary.md#workbench) must then open the addon and rewrite
  `apps/mod/tbd-framework/resourceDatabase.rdb`, because the game finds a non-script resource at a
  new path only through that database.

## Referenced by

- `TBD_UILayouts` names every layout by GUID and path.
- `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf` names the menu shells by GUID.
- `TBD_ListBox` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/TBD_ListBox.c` names
  `Common/TBD_ListRow.layout` by GUID as its default row.

## Boundaries

- Depends on: the textures in `apps/mod/tbd-framework/UI/Textures/TBD/`; the game's fonts and map
  layout; the script classes under `apps/mod/tbd-framework/Scripts/Game/TBD/` that the layouts name
  as components.
- Used by: the framework's screens, HUD and menus, through `TBD_UILayouts` and the menu config.
- Rules: every layout has one `TBD_UILayouts` constant and nothing instantiates a bare path;
  colours in a layout are placeholders, since the scripts paint every surface from `TBD_UITheme`;
  a layout and its `.meta` are committed together.
