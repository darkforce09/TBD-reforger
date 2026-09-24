# Doll model facade

One flat import point for the doll preview's CPU-side model: the region keys and states, the
instances and their colours, the two meshes, the picking and anchor functions, and the orbit
camera's two projections. It declares nothing of its own and hosts the model's unit tests.

## Contents

```text
apps/website/map-engine/src/doll/scene/model/
├── mod.rs  the module tree; re-exports the scene, picking and orbit projection items
└── tests/  unit tests for region keys, instances, mesh counts, projection, picks, colours, anchors
```

## Boundaries

- Depends on: `crate::doll::scene::instances`, `crate::doll::scene::mesh`,
  `crate::doll::interaction::picking` and `crate::camera::orbit::projection`, whose items it
  re-exports.
- Used by: `crate::doll::renderer`'s tests (`tests/pack_tests.rs` there imports the facade); no
  production code imports it, since the renderer, the readback check and the arsenal preview in
  `apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs` name the defining modules.
- Rules: a re-export only, so every item keeps its definition in the module it names; the tests
  pin the contract the renderer and the page rely on: 14 unique region keys, at least one
  instance a region, the exact cube and cylinder vertex and index counts, and the pick and anchor
  goldens (`tests/cases_1.rs`).
