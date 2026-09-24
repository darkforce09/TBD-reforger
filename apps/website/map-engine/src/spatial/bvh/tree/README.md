# BVH items under one path

`mod.rs` re-exports the sidecar, node, surface and traversal items of the parent BVH module
under one path, and mounts the BVH unit tests
(`apps/website/map-engine/src/spatial/bvh/tests/tree.rs`) as `tree::tests`, whose scene fixtures
other test modules of the crate share.

## Contents

```text
apps/website/map-engine/src/spatial/bvh/tree/
└── mod.rs  the module tree; re-exports the sidecar, node, surface and traversal items
```

## Boundaries

- Depends on: `crate::spatial::bvh` (`sidecar`, `node`, `surface` and `traversal`).
- Used by: tests only. The BVH unit tests import every item through
  `crate::spatial::bvh::tree::*`, and the test fixtures `Scene`, `cube` and `concat` under
  `crate::spatial::bvh::tree::tests` serve the tests of `crate::spatial::los::interior` and of the
  blueprint and section modules of `crate::world::architecture`. No production code imports this
  path.
- Rules: the module defines no item of its own; the test module is `pub(crate)`, which is what lets
  the other test modules reach its fixtures.
