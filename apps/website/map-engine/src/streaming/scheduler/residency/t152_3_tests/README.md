# Everon glyph and strip tests

Unit tests that load the committed Everon export into a `WorldResidency` and check the buffers it
composes against that data: the glyph lookup and the atlas keys, tree glyph packing, building
badges against an importance-aware oracle, landmark glyphs and fill de-emphasis, and the pier,
bridge rail and fence strips with their toggles.

## Contents

```text
apps/website/map-engine/src/streaming/scheduler/residency/t152_3_tests/
├── cases_1.rs  the cases: glyph lookup, atlas coverage, badges, landmarks, strip census and toggles
└── mod.rs      the module tree; the fixtures: the Everon residency, chunk drivers, the oracles
```

## Boundaries

- Depends on: the parent module's re-exports and test imports through `use super::*`
  (`WorldResidency`, `fence_prefab_lookup`, `building_prefab_lookup`, `narrow_prefab_rows`);
  `crate::overlay` for the class gates and the glyph keys; `crate::world` for the class codes, the
  footprint colours, the bridge fills and the strip geometry; `crate::streaming::buffers::revision`
  for `norm`; and the committed data it reads from disk: the manifest, prefab catalogue, chunk
  index and chunks under `assets_v2/terrains/everon/`, and `assets_v2/glyphs/manifest.json` with
  the atlas key file `assets_v2/glyphs/atlas/world-glyphs.json`.
- Used by: nothing outside the folder;
  `apps/website/map-engine/src/streaming/scheduler/residency/mod.rs` compiles it only in test
  builds (`#[cfg(test)] mod t152_3_tests;`).
- Rules:
  - at least 15 building prefabs map to badge glyphs (`g2_building_glyph_lookup_populated`), and
    all 84 tree and vegetation prefabs map to tree glyphs, every visible tree packing one
    (`tree_glyphs_pack_from_real_everon_data`);
  - the atlas key file and the glyph manifest list the same keys, and the atlas covers every
    prefab icon key and every building and badge key
    (`glyph_atlas_covers_every_requested_key_and_sources_agree`), each building class keyed
    `building-<class>` (`g1_building_icon_key_covers_normative_classes`);
  - the badge buffer holds exactly the badges the importance-aware oracle counts, from zoom -4 to
    3: below the badge band (zoom 1) only landmarks with an importance zoom draw, down to that zoom
    and not past it (`g3_zoom_gate_below_one_only_importance_landmarks`,
    `g4_class_r_badge_counts_match_oracle`, `g5_landmark_glyph_count_matches_oracle_at_z2`,
    `g6_lighthouse_instances_emit_building_lighthouse_glyph`,
    `t152_21_landmark_early_visibility`);
  - a landmark's bright fill gives way to its glyph below the badge band and returns above it
    (`t152_21_fill_deemphasis_handoff`);
  - fence and pier strips run along the footprint's long axis within 0.5 degrees
    (`t152_15_g2_orientation_parity_all_prefabs`);
  - the whole island at zoom 1.5 holds 2,299 pier strips and 144 bridges, each bridge with two
    rail strips, one deck fill and one casing fill; the fences toggle leaves piers and rails alone,
    and the buildings toggle removes piers and rails and leaves fences
    (`t152_15_pier_census_rails_casing_and_decoupling`).
