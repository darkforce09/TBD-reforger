# Everon glyph and strip tests

Unit tests that load the committed Everon export into a `WorldResidency` and check the buffers it
composes against that data: the glyph atlas keys, the zoom gate below the badge band and the
orientation of the fence and pier strips.

## Contents

```text
crates/streaming/chunk_draw_buffers/src/tests/everon_glyphs_and_strips/
├── cases_1.rs  the cases: atlas coverage, building icon keys, the low-zoom landmark gate, strip orientation
└── mod.rs      the module tree; the fixtures: the Everon residency, chunk drivers, the oracles
```

## Boundaries

- Depends on: `WorldResidency` from `crate::world_residency`, its
  `chunk_residency` field for the pinned ids and building footprints, and `prefab_catalog`'s
  `fence_prefab_lookup`, `building_prefab_lookup` and `narrow_prefab_rows`;
  `map_draw_lanes::zoom_gates` for the class gates and `label_layout::glyph_math` for the glyph
  keys; `prefab_catalog` for the class codes, the footprint corners and the payload reader;
  `road_network::cartographic_strip` for the strip geometry; and the committed data it reads
  from disk: the manifest, prefab catalogue, chunk
  index and chunks under `assets/terrains/everon/`, and `assets/glyphs/manifest.json` with
  the atlas key file `assets/glyphs/atlas/world-glyphs.json`.
- Used by: nothing outside the folder;
  `crates/streaming/chunk_draw_buffers/src/tests/mod.rs` compiles it only in test
  builds (`mod everon_glyphs_and_strips;`).
- Rules:
  - the atlas key file and the glyph manifest list the same keys, and the atlas covers every
    prefab icon key and every building and badge key
    (`glyph_atlas_covers_every_requested_key_and_sources_agree`), each building class keyed
    `building-<class>` (`g1_building_icon_key_covers_normative_classes`);
  - below the badge band (zoom 1) only landmarks with an importance zoom draw, down to that zoom
    and not past it (`g3_zoom_gate_below_one_only_importance_landmarks`);
  - fence and pier strips run along the footprint's long axis within 0.5 degrees
    (`t152_15_g2_orientation_parity_all_prefabs`).
