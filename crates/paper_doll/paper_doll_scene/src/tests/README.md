# Paper doll scene tests

The unit tests of the `paper_doll_scene` crate: the 14 equipment regions, the soldier's parts, the
unit mesh sizes, the state colours, the picks and the callout anchors, checked against their
goldens.

## Contents

```text
crates/paper_doll/paper_doll_scene/src/tests/
└── soldier_model_tests.rs  regions, parts, meshes, colours, picks, anchors and the perspective golden
```

## Boundaries

- Depends on: the crate's `soldier_parts`, `part_meshes` and `region_picking` modules and
  `camera_math` (`matrix4::perspective_no`, `matrix4::transform_vector`,
  `orbit::projection::view_proj_gl`).
- Used by: `cargo test -p paper_doll_scene`, through the `tests` module the crate root mounts.
- Rules: the cases run natively with no GPU and no browser.
