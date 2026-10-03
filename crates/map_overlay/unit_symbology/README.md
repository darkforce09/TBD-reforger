# Unit symbology

The `unit_symbology` crate: what a [slot](/documentation/glossary/n_to_z.md#slot), a vehicle, a
briefing marker and a squad draw as on the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map. It holds the three
side tints, the one role → glyph and alias → silhouette tables, the symbol atlas cells, the
marker vocabulary with its atlas and captions, and the squad link hairlines of the
[ORBAT](/documentation/glossary/n_to_z.md#orbat).

## Contents

```text
crates/map_overlay/unit_symbology/
├── Cargo.toml  the package: `map_draw_lanes`, `orbat_slot_ids`, `render_primitives`; layout tier 2
└── src/        the classification, symbol atlas, markers, squad links and prelude
```

## How it works

The symbology is bespoke: five unit roles, three vehicle kinds and three side tints, with no
MIL-STD-2525 or APP-6 frames. `classification::unit_role_class` and `vehicle_kind_for_alias`
normalise a role, kit or alias and fall back to the rifleman disc and the light wheeled hull.
`symbol_atlas` rasterises the two-cell slot atlas (ring, disc) and appends 15 white-on-alpha
cells: five role glyphs, the same five selected, three vehicle silhouettes and the comment
bubble with and without its selection ring. `markers` maps every marker `icon` alias of the
mission schema to one of 11 glyphs, folding case and separators, and its atlas shares cells 0
and 1 with the slot atlas. `squad_links` emits a leader → member hairline per member, tinted by
side, and offsets only the squads a drag touches.

## Getting started

Run from the repository root:

```bash
cargo test -p unit_symbology   # marker vocabulary, atlas and caption cases, squad link cases
```

## Public surface

- `classification::{SIDE_BLUFOR_RGBA, SIDE_OPFOR_RGBA, SIDE_INDFOR_RGBA, side_rgba,
  side_tints_rgba_bytes, UnitRoleClass, VehicleKind, unit_role_class, vehicle_kind_for_alias}`.
- `symbol_atlas::{build_slot_atlas, extend_atlas_with_unit_glyphs, WidenedSlotAtlas}` and the
  cell offsets.
- `markers::{MarkerGlyph, MARKER_GLYPH_COUNT, marker_glyph_for_alias, build_marker_slot_atlas,
  pack_marker_caption_bytes}`.
- `squad_links::{SquadLinkInput, build_squad_link_segments, pack_squad_link_drag_preview}`;
  `prelude`.

## Boundaries

- Depends on: `map_draw_lanes` (`zoom_gates::px_to_m_at_zoom` for captions),
  `render_primitives` (`text` layout and metrics), `orbat_slot_ids` (`SlotUid`, the
  squad link inputs' slot ids).
- Used by: `overlay_instances`; `symbology_layers_gpu` (the slot and marker lanes);
  `mission_editing_session` (the editing lanes and picking); the Mission Creator's document host,
  marker dock, canvas
  mount and select tool, and the mortar map picker, in `apps/frontend/`.
- Rules: every marker alias of the schema maps to a glyph (`every_schema_alias_maps`); marker
  atlas cells 0 and 1 equal the slot atlas (`marker_atlas_cells_0_and_1_match_slot_atlas`); the
  side tints are pinned in `src/classification.rs` by
  `cargo xtask verify editor-orbat-coherency`; map overlay tier 2
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Map symbology](/documentation/design_system/map_symbology.md) — the unit, vehicle and marker
  symbols and side tints, and how the game draws the same markers.
- [Design tokens](/documentation/design_system/design_tokens.md) — the palette the side tints
  come from.
