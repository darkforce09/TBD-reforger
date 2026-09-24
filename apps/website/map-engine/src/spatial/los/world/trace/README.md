# World line-of-sight query types

A second path to the world occluder and the types its queries answer with: `mod.rs` re-exports
`WorldOccluder` and the verdict, coverage, block policy and crossing types that
`apps/website/map-engine/src/spatial/los/world/coverage_1.rs` defines, which the parent module
also re-exports.

## Contents

```text
apps/website/map-engine/src/spatial/los/world/trace/
└── mod.rs  the module tree; re-exports `WorldOccluder`, `map_to_engine` and the query types
```

## Boundaries

- Depends on: `crate::spatial::los::world::coverage_1` (`BlockPolicy`, `Coverage`,
  `DEFAULT_BLAS_CAP_BYTES`, `Fidelity`, `PrefabOccluder`, `Wanted`, `WorldEvent`, `WorldLos`,
  `WorldVerdict`, `map_to_engine`) and `crate::spatial::los::world::state` (`WorldOccluder`).
- Used by: nothing; callers import the same items from `crate::spatial::los::world`, its
  `coverage_1` and its `state`.
- Rules: the module defines nothing of its own, so every item it names stays defined in the parent
  folder.
