# Geometry primitives tests

Unit tests of `geometry_primitives`, each file declared by the module it tests through `#[path]`.

## Contents

```text
crates/geometry/geometry_primitives/src/tests/
├── axis_aligned_box.rs  the union of two boxes is component-wise
└── rigid_transform.rs   rotation directions, inverses, quaternion round trips, nested composition precision
```

## Boundaries

- Depends on: the module each file tests (`crate::<module>`).
- Used by: `cargo test -p geometry_primitives`.
- Rules: every expected value is written out by hand; a nested composition keeps sub-micrometre
  precision (`nested_composition_keeps_sub_micrometre_precision`).
