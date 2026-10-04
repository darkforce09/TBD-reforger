# Right dock markers

The right dock's Markers tab: the closed marker icon vocabulary read from the
[mission](/documentation/glossary/g_to_m.md#mission) schema, the icon picker that arms a marker
placement, the list of authored markers and the selected marker's form. A marker belongs to one
side's briefing.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/markers/
├── glyph_preview.rs  `marker_glyph_svg`: the SVG preview of one map glyph, beside each picker row
├── mod.rs            the module tree; re-exports `markers_panel`
└── panel.rs          `markers_panel`: icon search and picker, the authored markers, the selected one's form
```

## How it works

The icon vocabulary is the state layer's `marker_icons`
(`crates/frontend/workspaces/mission_creator_state/src/marker_icons.rs`): `marker_icons` reads the closed
`$defs/marker.icon` enum once, in schema order, from the one schema embed
(`contracts/definitions/mission.schema.json`). `marker_icon_is_authorable` is
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

- Depends on: the state layer's `marker_icons` and `zones::humanize_token` in
  `crates/frontend/workspaces/mission_creator_state/src/`; the outliner's `ROW` and
  `ROW_ACTIVE`; `begin_place_marker` and `armed_marker_icon` from
  `bridge::host_state::armed_placement`; and, in the browser build,
  `mission_editing_commands::hosted_commands` (`marker_rows`, `marker_count`, `set_marker_icon`,
  `set_marker_label`, `set_marker_position`, `remove_marker`), and `unit_symbology::markers` (`MarkerGlyph`,
  `marker_glyph_for_alias`, `MARKER_GLYPH_COUNT`).
- Used by: the Markers tab of `DockRight` in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/shell/layout.rs`;
  the tests in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/tests/dock_right/`.
- Rules: that folder's `marker_icons_and_briefing.rs` and the browser build hold these: the icon
  list is the schema's own and only its aliases are authorable (`the_icon_list_is_the_schemas_own`,
  `only_schema_aliases_are_authorable`); the picker has one row per glyph
  (`picker_has_one_row_per_canonical_icon`, which also checks each canonical slug folds to a
  distinct glyph), and the canonical slug table is sized by `unit_symbology`'s `MARKER_GLYPH_COUNT`,
  so a glyph set of another size fails to compile; marker writes go to
  the side's briefing, never to a root marker map
  (`marker_writes_go_to_the_briefing_not_the_root_map`).

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/right_asset_palette.md) — the Markers tab.
- [Mission Creator feature inventory: placement](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/placement.md) — dropping a briefing marker.
