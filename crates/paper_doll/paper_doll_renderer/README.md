# Paper doll renderer

The `paper_doll_renderer` crate: the GPU renderer of the
[arsenal](/documentation/glossary/a_to_f.md#arsenal)'s 3D paper doll in the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator). `PaperDollRenderer` creates
the GPU of its own canvas through `gpu_device::GpuContext`, draws the soldier of
`paper_doll_scene` as instanced cubes and one cylinder with depth testing whenever something
changed, takes the Arsenal's region states, hover, turns and resizes, answers picks and callout
anchors, and runs a byte-exact offscreen readback self-check.

## Contents

```text
crates/paper_doll/paper_doll_renderer/
├── Cargo.toml  the package: `paper_doll_scene`, `gpu_device`, `camera_math`; `wgpu` and the browser crates on wasm32; tier 3, wasm32
└── src/        the renderer, its frame, pipeline, draws, self-check, instance packing, shader and tests
```

## How it works

The Arsenal host sizes the canvas in device pixels, awaits `PaperDollRenderer::create`, resizes
it to the container's CSS size, and then drives it: pointer drags turn it, pointer moves pick and
hover, clicks pick, the loadout pushes one state byte per region, and every animation frame calls
`render`, which draws only when something changed. The self-check is the browser's proof that the
pipeline, the depth test and the colours are right. The source README details the frame, the
instance stream and the probes.

## Getting started

Run from the repository root:

```bash
cargo test -p paper_doll_renderer                                              # the instance stream tests
cargo clippy -p paper_doll_renderer --target wasm32-unknown-unknown -- -D warnings  # the browser half
```

## Configuration

None: no feature and no environment variable. The browser half is selected by the `wasm32`
target; `?force=webgl` on the page URL makes the Arsenal host ask for WebGL2.

## Public surface

- `PaperDollRenderer` (WebAssembly) and `Error` and `Result`, at the root.
- `instance_packing`: `INSTANCE_STRIDE`, `InstanceStreams` and `pack_instances`.
- `prelude`: the names an Arsenal host imports.

## Boundaries

- Depends on: `paper_doll_scene`, `gpu_device`, `camera_math`, `bytemuck`, `thiserror`; `wgpu`,
  `web-sys`, `js-sys` and `wasm-bindgen-futures` in the WebAssembly build.
- Used by: the frontend's Arsenal host (`apps/frontend/src/workspaces/editor/arsenal/doll.rs`).
- Rules: paper doll category, tier 3 (`cargo xtask verify crate-tiers`); one of the crates
  allowed `wgpu`; a file that names a GPU or browser type gates itself on `wasm32`, so
  `cargo test -p paper_doll_renderer` runs natively.

## Related documentation

- [Paper doll crates](/crates/paper_doll/README.md) — the scene and the renderer together.
- [GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) — the
  graphics crates the renderer creates its GPU with.
