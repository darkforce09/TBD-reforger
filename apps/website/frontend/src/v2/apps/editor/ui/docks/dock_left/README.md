# Left dock

The [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s left dock: the Layers tab,
which shows the editor layers tree, searches the [mission](/documentation_v2/glossary.md#mission)
and narrows a selection, and the Locations tab, which holds the author's camera bookmarks and the
terrain's named locations. The module root is
`apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left.rs`: it declares these files, holds the
tab labels and `collapse_chevron`, and re-exports the items below.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left/
├── bookmarks.rs        `Bookmarks`: named camera views in local storage; add, rename and remove
├── camera.rs           the live camera read, the fly-to and the named-location load
├── document_search.rs  the mission search, whether a hit can be selected, and the selection chips
├── places.rs           `LeftTab`, `NamedPlace`, the one query predicate and the layers tree filter
├── view/               the `full_dock!` and `places_body!` view fragments `DockLeft` expands
└── view.rs             `DockLeft`: the dock's signals, the search effects and the collapsed stub
```

## How it works

`DockLeft` takes the layers tree the editor rebuilds after every document change, the selection,
the active layer and the collapse flag. Collapsed, it draws a 24 px (`STUB_PX`) stub holding only
the chevron; expanded, it draws the full dock from `view/`, on the Layers tab by default.

On the Layers tab one search box feeds three things. `filter_outliner` prunes the tree: a node whose
own label matches keeps its whole subtree, and a node kept only for a matching descendant keeps
just the matching paths. `search_document` runs the same text over the map engine's document index
and lists up to `MAX_DOC_HITS` (200) hits under "Found N"; a hit is a live button only when the
selection router the editor registers with the validation panel resolves its id
(`hit_is_routable`), and an inert hit says why in its tooltip. With two or more entities selected,
`selection_facets` offers chips that narrow the selection by type or by faction, each a proper
subset of it. Above the box, the "Placing into:" strip names the layer the next placement files
into: the active layer, else the first top-level layer, else "a new layer". The header's add
button creates a layer as a child of the active one, and dropping a dragged folder on the header
moves it to the top level.

The Locations tab loads the named locations once, on first open, sorted by name. Its bookmarks
live in local storage under `tbd-mc-editor-bookmarks`: version 1, at most 200, newest first, with
the trimmed, case-folded name as the identity, so an empty or duplicate name is refused. Clicking a
row flies the camera there, at the bookmark's zoom or at the current one.

Every list in the dock filters through `matches_query`, which runs the asset palette's search
grammar (plain text, `*`, `?`, `/…/`, `class:` and `mod:`), so the tree, the search, the bookmarks
and the locations agree on what the box means. Bookmarks, fly-to and narrowing the selection are
view and selection state: none of them edits the document or adds an undo step.

## Public surface

- `DockLeft`: the dock component, which
  `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs` re-exports and
  `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` mounts.
- The pure helpers the module root re-exports (`Bookmarks`, `filter_outliner`, `search_document`,
  `selection_facets`, `matches_query` and the rest) serve only the dock's own tests.

## Boundaries

- Depends on:
  - in the editor: `shell::layout` (`DOCK_L`, `STUB_PX`); the outliner's `OutlinerNode`,
    `virtual_tree` and `create_layer` in `apps/website/frontend/src/v2/apps/editor/ui/outliner/`;
    the asset catalog's `filter_catalog` and `search_empty_message`, for the search grammar; the
    validation panel's `subject_id_routes` and `route_select_by_subject_id`; and
    `bridge::host_state::entity_selection::set_selection_ids`;
  - `crate::v2::core::ui` (`MaterialIcon`);
  - `website_map_engine`: `DocEntity` and `DocKind` from
    `data::store::operations::document_index`; `document_entities`, `selection_entities`,
    `complete_layer_drop_onto_root` and `cancel_layer_drag` from `editing::hosted_commands`;
    `camera_snapshot`, `fly_to` and `named_locations` from `streaming::host`;
  - the browser's local storage, through `web_sys`.
- Used by: the chrome re-export and the editor page above; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_left/`; and the outliner smoke
  test in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_palette.rs`,
  which finds the dock by its "Layers" and "Locations" tabs.
- Rules, held by the tests in `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_left/`:
  - every body that reaches the engine, the selection or local storage compiles for `wasm32`
    only and answers empty or `None` elsewhere, so the helpers run in the native tests;
  - the bookmark key keeps its `tbd-` namespace and its version stamp, and a stored blob is
    cleaned on load (`bookmarks_key_is_namespaced_and_versioned` and
    `migrate_bookmarks_drops_junk_rows` in `bookmarks_places_and_tabs.rs`);
  - bookmark and fly-to code never reaches the undo history
    (`bookmarks_and_fly_to_are_not_document_edits`), and neither does a selection chip
    (`narrowing_the_selection_is_not_undoable` in that folder's `document_search.rs`);
  - a hit is a button exactly when the router would select it
    (`a_hit_row_is_a_live_affordance_iff_the_click_would_select`), and the document index covers
    every collection an author can place into (`the_index_covers_every_placeable_collection`),
    both in the same test file;
  - the tab header fits the 240 px dock (`the_header_row_fits_the_dock` in
    `dock_density_and_search.rs`);
  - `test_source.rs` there reassembles the dock's production files, fragments expanded, for the
    source checks, so a new file here joins its list.

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the dock layout and the outliner interactions.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the left sidebar's features, one entry each.
