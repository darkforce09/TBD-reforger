# Map label and glyph math

The rules that decide which map labels draw at a zoom and how world glyphs are sized, angled and
packed: collision declutter for peak and generic labels, the importance and zoom bands of town
labels, and the 20-byte icon instance every glyph lane uploads.

## Contents

```text
apps/website/map-engine/src/overlay/symbology/labels/
├── declutter.rs   `LabelSpec`, `declutter`: drop labels within a zoom-scaled distance
├── glyph_math.rs  glyph sizes, yaw encoding, colour packing, building keys, icon packing
├── importance.rs  `LocationLabel` and the town-label bands: zoom range, kind, importance, fade
├── mod.rs         the module tree; `glyph_math` and `importance` need the `streaming` feature
└── tests/         unit tests for each file's rules and invariants
```

## How it works

- **Declutter.** `declutter` drops blank labels, sorts by importance (then by id), and keeps a
  label only when no kept label of equal or higher importance lies within
  `min_label_distance_m(zoom)`, which is `MIN_LABEL_PX` (48 px) at zoom 0 scaled by `2^-zoom`.
  `declutter_invariant_holds` checks that no two drawn labels sit closer than that distance.
- **Town labels.** `should_draw_town_label` draws a location only inside
  `TOWN_LABEL_MIN_ZOOM` (−4.5) to the ceiling (`TOWN_LABEL_FADE_END`, 3.0, while
  `TOWN_LABEL_FADE_ENABLED`); only towns, villages and airports qualify, and localities from zoom
  0; below `TOWN_LABEL_WIDE_ZOOM` (−3.0) only importance 0.70 and up; a trimmed name shorter than
  two characters never draws; and a label yields to a more important neighbour closer than its own
  `town_declutter_threshold_m`. `town_label_fade_alpha`
  fades from `TOWN_LABEL_MAX_ZOOM` (2.0) to the fade end.
- **Glyphs.** Sizes are world metres, a base size in pixels over `2^REF_ZOOM`
  (`crate::overlay::lod::REF_ZOOM`), with tree glyphs scaled by height (×1.0 to ×1.5) and a
  minimum on-screen size (`size_with_min_px`). `deck_angle_for_rotation_deg` turns a clockwise
  compass yaw into the screen's counter-clockwise angle, and `yaw_to_snorm16` wraps it into
  (−180°, 180°] rather than clamping. `pack_icon_instance` writes the 20-byte instance
  (`ICON_INSTANCE_STRIDE`): position, size, yaw, glyph index and packed RGBA. The building class
  table (`BUILDING_CLASSES`) maps to a footprint glyph key or, for military, tower and bunker, a
  badge key.

## Boundaries

- Depends on: `crate::overlay::lod::REF_ZOOM`; `serde` for `LocationLabel`.
- Used by:
  - `crate::overlay::symbology::text_packing` (label strings to glyph instances);
  - the streaming buffers (`crate::streaming::buffers::glyphs`, `packer` and `revision`) and the
    streaming host's `named_locations`;
  - the location loaders in `crate::world::environment::locations` (towns and peaks);
  - the town-label map verification in
    `tools_v2/developer-tools/src/map_verification/labels/town_labels.rs`.
- Rules: yaw wraps rather than clamps, so 270° and 180° stay distinct
  (`yaw_snorm16_wraps_not_clamps`, `world_rotation_270_differs_from_180` in
  `tests/glyph_math_tests.rs`); an icon instance is 20 bytes (`pack_icon_instance_is_20_bytes`);
  the declutter invariant holds on a mixed fixture (`g4_invariant_on_randomish_fixture` in
  `tests/declutter_tests.rs`).
