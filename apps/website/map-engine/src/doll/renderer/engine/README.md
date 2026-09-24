# Doll engine re-export

A one-line module that re-exports `DollEngine`, the doll preview's browser renderer, under
`crate::doll::renderer::engine`. It compiles only for wasm32 with the `render` feature, like the
renderer files it points at.

## Contents

```text
apps/website/map-engine/src/doll/renderer/engine/
└── mod.rs  re-exports `DollEngine` from `crate::doll::renderer::lifecycle_1`
```

## Boundaries

- Depends on: `crate::doll::renderer::lifecycle_1`, which defines `DollEngine`.
- Used by: nothing; the arsenal preview in
  `apps/website/frontend/src/v2/apps/editor/arsenal/doll.rs` and the readback check in
  `apps/website/map-engine/src/diagnostics/readback/doll.rs` name
  `doll::renderer::lifecycle_1::DollEngine` directly.
- Rules: the module declares nothing of its own; the struct and its methods stay in
  `lifecycle_1.rs` and `lifecycle_2.rs` beside it.
