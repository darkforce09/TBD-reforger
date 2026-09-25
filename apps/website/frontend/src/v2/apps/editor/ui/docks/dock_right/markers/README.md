# Right dock markers

The right dock's Markers tab: the closed marker icon vocabulary read from the
[mission](/documentation_v2/glossary.md#mission) schema, the icon picker that arms a marker
placement, the list of authored markers and the selected marker's form. A marker belongs to one
side's briefing.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/markers/
├── icons.rs  the schema's marker icons, the picker's one row per glyph, and the glyph previews
├── mod.rs    the module tree; re-exports the icon vocabulary and `markers_panel`
└── panel.rs  `markers_panel`: icon search and picker, the authored markers, the selected one's form
```

## How it works

`marker_icons` reads the closed `$defs/marker.icon` enum once, in schema order, from the schema the
zones panel embeds (`contracts_v2/definitions/mission.schema.json`). `marker_icon_is_authorable` is
the exact, case-sensitive test every marker write passes, and `default_marker_icon` is the enum's
first entry. The picker shows one row per glyph the map draws (`CANONICAL_MARKER_SLUGS`, 11 of
them): it folds every alias through the map engine's `marker_glyph_for_alias`, so the preview and
the map agree, and a row press arms that glyph's canonical slug. "Search icons" matches a row's
label, slug and aliases, with underscores read as spaces.

The authored list reads the map engine's `marker_rows`, one row per marker with its side and
position. Selecting one opens its form, "Marker <id> — <side>", with "Type", "Text", "Position (x,
z metres)" and "Delete marker"; every write addresses the marker by its faction id and marker id
and bumps `doc_tick`. The panel renders only in the browser build; the native `markers_panel` draws
nothing.

## Boundaries

- Depends on: `MISSION_SCHEMA` and `humanize_token` from the zones panel in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/`; the outliner's `ROW` and
  `ROW_ACTIVE`; `begin_place_marker` and `armed_marker_icon` from
  `bridge::host_state::armed_placement`; and, in the browser build, `website_map_engine`'s
  `editing::hosted_commands` (`marker_rows`, `marker_count`, `set_marker_icon`, `set_marker_label`,
  `set_marker_position`, `remove_marker`) and `overlay::symbology::markers` (`MarkerGlyph`,
  `marker_glyph_for_alias`, `MARKER_GLYPH_COUNT`).
- Used by: the Markers tab of `DockRight` in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/shell/layout.rs`;
  `begin_place_marker` in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/palette_arming.rs`,
  which refuses an icon `marker_icon_is_authorable` rejects; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/`.
- Rules: that folder's `marker_icons_and_briefing.rs` and the browser build hold these: the icon
  list is the schema's own and only its aliases are authorable (`the_icon_list_is_the_schemas_own`,
  `only_schema_aliases_are_authorable`); the picker has one row per glyph
  (`picker_has_one_row_per_canonical_icon`), and the browser build fails to compile when
  `CANONICAL_MARKER_GLYPH_COUNT` differs from the engine's `MARKER_GLYPH_COUNT`; marker writes go to
  the side's briefing, never to a root marker map
  (`marker_writes_go_to_the_briefing_not_the_root_map`).

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) — the Markers tab.
- [Mission Creator feature inventory: placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) — dropping a briefing marker.
