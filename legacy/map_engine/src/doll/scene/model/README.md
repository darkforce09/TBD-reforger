# Doll model facade

One flat import point for the doll preview's CPU-side model: the region keys and states, the
instances and their colours, the two meshes, and the picking and anchor functions. It declares
nothing of its own and hosts the model's unit tests, which take the orbit camera's projections
from `camera_math::orbit::projection`.

## Contents

```text
legacy/map_engine/src/doll/scene/model/
├── mod.rs  the module tree; re-exports the scene and picking items
└── tests/  unit tests for region keys, instances, mesh counts, projection, picks, colours, anchors
```

## Boundaries

- Depends on: `crate::doll::scene::instances`, `crate::doll::scene::mesh` and
  `crate::doll::interaction::picking`, whose items it re-exports; its tests also use
  `camera_math::orbit::projection` and `camera_math::matrix4`.
- Used by: `crate::doll::renderer`'s tests (`tests/pack_tests.rs` there imports the facade); no
  production code imports it, since the renderer, the readback check and the arsenal preview in
  `apps/frontend/src/v2/apps/editor/arsenal/doll.rs` name the defining modules.
- Rules: a re-export only, so every item keeps its definition in the module it names; the tests
  pin the contract the renderer and the page rely on: 14 unique region keys, at least one
  instance a region, the exact cube and cylinder vertex and index counts, and the pick and anchor
  goldens (`tests/cases_1.rs`).
