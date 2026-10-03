# Paper doll shader

The WGSL program of the paper doll renderer, the 3D mannequin preview in the
[arsenal](/documentation/glossary/a_to_f.md#arsenal). The map's own shaders live in the graphics
crates, under `crates/graphics/render_primitives/src/shaders/`.

## Contents

```text
crates/paper_doll/paper_doll_renderer/src/shaders/
└── doll.wgsl  `vs_doll` and `fs_doll`: instanced mannequin parts with Lambert-plus-ambient shading
```

## How it works

`vs_doll` multiplies each vertex by its instance's model matrix and the camera's `mvp` from the
group-0 uniform (80 bytes: the matrix and a `params` vector) and carries the normal and colour on.
`fs_doll` shades the colour by a fixed light direction with 0.42 ambient; when `params.x` is above
0.5 it returns the colour unlit, which the byte-exact self-check needs. Vertex locations 0 and 1
are position and normal (24-byte stride); locations 2 to 6 are the instance's four matrix columns
and its colour (80-byte stride).

## Boundaries

- Depends on: nothing; the file is WGSL source.
- Used by: `crates/paper_doll/paper_doll_renderer/src/pipeline.rs`, which embeds it with
  `include_str!`, compiles it into the doll's shader module and names its entry points in the
  render pipeline.
- Rules: the uniform size, binding, vertex locations and strides match their Rust owners
  (`UNIFORM_SIZE` and the vertex layouts in `pipeline.rs`, `INSTANCE_STRIDE` in
  `crates/paper_doll/paper_doll_renderer/src/instance_packing.rs`); no native test compiles the
  file, so a mismatch shows only when a browser builds the pipeline.
