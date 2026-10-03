# Label layout

The `label_layout` crate: which labels the map draws at a zoom, and the glyph instances they draw
as. It declutters generic map labels by distance and importance, decides which towns of a
terrain's `locations.json` draw and how they fade, sizes and keys the world's object glyphs, and
packs decluttered labels into monospaced glyph instances of the renderer's baked ASCII atlas.

## Contents

```text
crates/map_overlay/label_layout/
├── Cargo.toml  the package: `newtype_ids`, `render_primitives`, `serde`; layout tier 1
└── src/        the declutter, town importance, glyph sizing, label packing, ids and prelude
```

## How it works

`declutter::declutter` drops blank labels, ranks the rest by importance and then by the lower
`LabelId`, and keeps a label only when no kept label of equal or higher importance stands within
`min_label_distance_m` (48 px at the zoom). `importance::should_draw_town_label` admits a town
inside the zoom band, with a kind the zoom allows, when no more important location stands within
its importance-scaled threshold; `town_label_fade_alpha` fades towns out above zoom 2.
`text_packing::pack_label_glyphs` declutters and lays the survivors out through
`render_primitives::text::layout`, dropping the importance column before the renderer sees it.
`glyph_math` sizes world object glyphs in metres at `render_primitives::text::scale::REF_ZOOM`,
parses hex tints, turns an exported clockwise rotation into the screen angle, and names the atlas
key of each building class.

## Getting started

Run from the repository root:

```bash
cargo test -p label_layout   # declutter, importance, glyph sizing, packing and id cases
```

## Public surface

- `declutter::{LabelSpec, declutter, declutter_invariant_holds, min_label_distance_m}`.
- `importance::{LocationLabel, should_draw_town_label, declutter_town_labels,
  town_label_fade_alpha}` and the town label constants.
- `glyph_math`: the glyph size constants, `deck_angle_for_rotation_deg`, `glyph_size_meters`,
  `badge_size_meters`, `hex_to_rgba` and the building icon keys.
- `text_packing::{pack_label_glyphs, glyphs_from_specs, to_glyph_specs}`.
- `label_ids::{LabelId, LocationId}`, and `prelude`.

## Boundaries

- Depends on: `render_primitives` (`text::layout`, `text::metrics`, `text::pack`, `text::scale`),
  `newtype_ids` (the ids), `serde` (the location rows); `serde_json` in tests.
- Used by: `place_names`, whose packers lay out the town, road and height labels;
  `map_asset_loading`'s location loader (`crates/streaming/map_asset_loading/src/environment/location_labels/`);
  `chunk_draw_buffers`' draw buffers and glyph lookup; `map_streaming_host`; and the
  town-label verification of the developer tools
  (`tools/map_assets/map_asset_verification/src/labels/`).
- Rules: the committed Everon label data draws without a tofu glyph
  (`g3_committed_label_data_no_tofu`) and keeps its location id bytes through a typed round trip
  (`the_committed_location_rows_keep_their_id_bytes`); a world rotation covers the whole compass
  (`every_world_rotation_of_the_compass_gets_its_own_facing`); map overlay tier 1
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Map symbology](/documentation/design_system/map_symbology.md) — the labels and glyphs the map
  draws.
