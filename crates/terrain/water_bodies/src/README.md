# Water bodies source

The source of `water_bodies`: the bathymetry and inland water readers, the sea fill mesh, the
error they share, and the crate root that declares them.

## Contents

```text
crates/terrain/water_bodies/src/
├── error.rs    `Error` and `Result`: a water file's `BinaryError` behind one type
├── lib.rs      the crate root: module header, `mod` lines and re-exports
├── mesh.rs     `compose_sea_mesh`: the sea band's rings triangulated into the sea fill mesh
├── prelude.rs  the names most readers import
├── tests/      unit tests for the bathymetry levels, the mask, the suffix plan and the archive
└── vectors.rs  the bathymetry pyramid and its water mask, the suffix plan, and the inland archive
```

## How it works

`vectors` reads byte slices the caller fetched and answers questions about them; `mesh` turns the
sea band of `terrain_relief` into a fill mesh shaped by `render_primitives`. Neither fetches nor
uploads.

## Boundaries

- Depends on: `terrain_relief`, `render_primitives`, `world_file_formats`, `rkyv`, `bytemuck` and
  `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
