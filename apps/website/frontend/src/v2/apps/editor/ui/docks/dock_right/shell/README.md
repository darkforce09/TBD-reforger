# Right dock shell

The frame of the right dock: `DockRight` with its seven-tab strip, the body of each tab, the
collapsed stub, the Factions tab's asset browser, and the hook through which the editor's
selection router selects a zone in the Zones tab.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/shell/
├── factions_panel.rs  the Factions tab: side chips, asset or object search, faction or objects tree
├── layout.rs          `DockRight`: its signals and effects, the tab strip, the tab bodies, the stub
└── mod.rs             the module tree; re-exports `DockRight`; tab strip classes and glyphs, `ZONES_TAB`, zone hook
```

## How it works

`DockRight` takes what the editor page loads and owns: the character and vehicle catalog states,
the raw [registry](/documentation_v2/glossary.md#registry) rows, the registry failure flag and fetch
generation, `doc_tick`, the faction manager's open flag, `active_side`, `objects_mode` and the
collapse flag. Collapsed, it draws only the chevron in a 24 px stub. Expanded, its strip holds seven
20 px glyph tabs, each named by its title and `aria-label`, in the order Factions, Vehicles, Zones,
Compositions, Triggers, Favourites and Markers, then the "Manage factions" verb, which opens the
faction manager, and the chevron. The tab indices, which `EdenSubmode::from_tab` reads, differ from
that order: Markers is index 2 and Zones is `ZONES_TAB`, 3.

| Tab | Body |
|---|---|
| Factions | "Asset Browser": the side chips, a search box, the merged faction tree or the objects tree |
| Vehicles | every placeable vehicle, a search box and, in the browser build, "Place with crew" |
| Zones | the inspector's `zones_panel`, with the zone selection this component owns |
| Compositions, Triggers, Markers | the panels of the sibling folders of the same names |
| Favourites | the "Favourites" and "Recently placed" lists, one at a time |

Each tree keeps its own set of collapsed folders, seeded by `collapsed_seed`; the faction tree's
set reseeds when the side changes, and a search draws its filtered tree fully open. When the
registry fetch fails, the browser build probes `GET /api/v1/registry?limit=1&offset=0`, and a 404
answer makes the failure view say that no modpack is configured.

A zone's selection lives in this component, apart from the
[slot](/documentation_v2/glossary.md#slot) selection. At mount `DockRight` installs a hook
(`install_select_zone`) that selects a zone, raises the Zones tab and expands the dock; the editor's
selection router calls `route_select_zone` for a zone subject, which reports whether a dock was
there to take it. Cleanup removes the hook only while it is still the registered one, so an older
mount's cleanup never clears a newer mount's hook. `DockRight` installs the recently placed recorder
the same way.

## Boundaries

- Depends on: the sibling folders of `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/`
  through its scope (the chips, the palette rows, the favourites, the recent list and the
  Compositions, Triggers and Markers panels); `collapse_chevron` from the left dock; `DOCK_R` and
  `STUB_PX` from `crate::v2::apps::editor::shell::layout`; `zones_panel` from the inspector; the
  asset catalog's `CatalogState`, `build_faction_catalog_tree`, `build_object_catalog_tree`,
  `filter_catalog` and `search_empty_message`; `editor_context` for the crew preference; `api_get`
  and `AuthStore` from `crate::v2::core` for the registry probe.
- Used by: the module root `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right.rs`, which
  re-exports `DockRight` for `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs` and
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, and the zone routing for the
  selection router in `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`;
  the tests in `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/`; the outliner
  smoke test in
  `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_palette.rs`, which
  opens the Factions tab by its `aria-label`.
- Rules: that tests folder holds these: the strip fits the 240 px dock and every glyph tab keeps
  its name (`the_tab_strip_fits_the_dock` and `every_glyph_tab_keeps_its_name` in
  `tab_strip_budget.rs`); the hook selects the zone and shows its tab, and the Zones index is
  stated once (`the_hook_selects_the_zone_and_shows_it` and `the_zones_tab_index_is_stated_once` in
  `zone_selection_seam.rs`); an older mount's cleanup leaves a newer hook in place
  (`an_older_owners_cleanup_does_not_clobber_a_newer_registration` in `zone_hook_lifecycle.rs`).
