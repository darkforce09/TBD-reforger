**Status:** live

# Mission Creator feature inventory

Every feature of the [Mission Creator](/documentation_v2/glossary.md#mission-creator), one area
per file: what the mission maker does, what the code does in reply, and whether the committed code
ships it. Developers and agents read it to find a feature's code and state, and the Eden gap
analysis reads it to pair each feature with its Eden counterpart.

## Contents

```text
documentation_v2/website/frontend/apps/editor/feature_inventory/
├── attributes_and_settings.md          the Attributes dialog: transform, identity, states, arsenal, vehicles
├── bottom_toolbelt.md                  the mode toolbar, the status bar read-outs, the edge grid references
├── data_persistence_and_compile.md     the local draft, boot hydrate, conflict dialog, compile, tab lock
├── editor_route_loading.md             the editor route, its access tier and the boot overlay
├── feds_schema.md                      the entry schema: feature IDs, entry format, terms, gap rows
├── keyboard_shortcuts.md               every key binding, the field guard and the shortcut list
├── left_sidebar.md                     the left dock: editor layers, Locations, and the ORBAT tree
├── map_basemap_and_world_objects.md    the basemap, the world-object layers and their switches
├── map_viewport_and_camera.md          the map view: pan, zoom, grid, centring, cursor, terrain
├── performance_at_scale.md             bulk paste, windowed trees, clustering and load at scale
├── placement.md                        placing from the palette and by click
├── right_asset_palette.md              the asset palette's tabs, catalog and search
├── selection.md                        click, marquee, modifier and tree selection
├── shell_route_and_layout.md           the chromeless layout, the invalid-id banner, the FPS read-out
├── top_command_strip.md                menus, title, undo, save, export, environment, settings
└── transform_and_delete.md             move, rotate, snap and delete
```

## How it works

Each area file is a [feature doc](/documentation_v2/standards/templates/feature_doc.md) whose
Behaviour holds the area's entries, in the format the [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md)
sets: a table of IDs and statuses, one `###` section per ID with numbered steps, and the area's
known discrepancies. IDs follow the schema's `{DOMAIN}-{SUBDOMAIN}-{NNN}` pattern and are never
reused; a feature the code has and the inventory lacks gets a new ID in its area's file.

| Status | Meaning |
|---|---|
| shipped | the committed code does what the entry says |
| partial | part of the entry works; the steps say which part does not |
| not built | nothing in the code does it, or a visible control has no action |

| Area | File | Entries | Verified against the code |
|---|---|---|---|
| MAP — viewport and camera | [map_viewport_and_camera.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md) | 7: 5 shipped, 2 partial | yes |
| MAP — basemap and world objects | [map_basemap_and_world_objects.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_basemap_and_world_objects.md) | 13: 5 shipped, 1 partial, 7 not built | yes |
| LEFT — left dock and ORBAT tree | [left_sidebar.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md) | 21: 18 shipped, 1 partial, 2 not built | yes |
| BOTTOM — toolbelt | [bottom_toolbelt.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md) | 9: 8 shipped, 1 not built | yes |
| ATTR — Attributes dialog | [attributes_and_settings.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) | 7: 6 shipped, 1 not built | yes |
| DATA — persistence and compile | [data_persistence_and_compile.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md) | 11: 8 shipped, 1 partial, 2 not built | yes |
| KEY — keyboard | [keyboard_shortcuts.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md) | 14: 13 shipped, 1 partial | yes |
| FILE — route and boot | [editor_route_loading.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/editor_route_loading.md) | 2: 1 shipped, 1 not built | yes |
| SHELL — route and layout | [shell_route_and_layout.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/shell_route_and_layout.md) | — | no |
| SEL — selection | [selection.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/selection.md) | — | no |
| XFORM — transform and delete | [transform_and_delete.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md) | — | no |
| PLACE — placement | [placement.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) | — | no |
| RIGHT — asset palette | [right_asset_palette.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) | — | no |
| TOP — command strip | [top_command_strip.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/top_command_strip.md) | — | no |
| PERF — behaviour at scale | [performance_at_scale.md](/documentation_v2/website/frontend/apps/editor/feature_inventory/performance_at_scale.md) | — | no |

An area marked "no" still holds entries whose names, statuses and evidence describe code the
repository does not hold (React components, Yjs, Deck.gl); its statuses are not evidence until the
area is verified and rewritten to the schema. Features Eden has no counterpart for carry the
`TBD-` domain inside their area: TBD-LAYER-001 in the left sidebar, TBD-CONFLICT-001 in data
persistence, TBD-SAVE-001 and TBD-EXPORT-001 in the top command strip.

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — every area: the page, docks,
  inspectors, input and browser session.
- [Map engine editing](/apps/website/map-engine/src/editing/) — the hosted commands, undo history,
  tools and persistence the areas call.
- [Map engine data](/apps/website/map-engine/src/data/) — the mission document and the payload
  compiler.

## Boundaries

- Depends on: the [feature entry schema](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md)
  and the [feature doc template](/documentation_v2/standards/templates/feature_doc.md); the
  committed code under the folders above; the ticket registry in `.ai/tickets/` for Open work.
- Used by: the Related documentation of the in-code READMEs under
  `apps/website/frontend/src/v2/apps/editor/` and of `apps/website/map-engine/src/editing/` and
  `apps/website/map-engine/src/data/store/`; the
  [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md),
  which pairs its rows with Eden's by ID; the [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)
  and the [decisions log](/documentation_v2/website/frontend/apps/editor/decisions.md).
- Rules: one area per file, each within 500 lines (`cargo xtask verify markdown-placement`);
  every file here has a Contents line (`cargo xtask verify readme-coverage`); an ID is never
  reused or renumbered; a status is read from the committed code, never from a ticket.

## Related documentation

- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  entry point to the roadmap, the specifications and the decisions.
- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the layout, the interaction contract and the shortcuts the areas are built against.
- [Eden interactions reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md)
  — the Eden catalogue the inventory is compared with.
