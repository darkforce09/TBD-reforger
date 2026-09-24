# Graphics engine

The `website-graphics-engine` crate: the web platform's WebGPU renderer, which knows no map
concept. It defines the frame vocabulary a caller describes a frame in, and supplies the render
pipelines, the WGSL shader, geometry and text packing, sprite culling and the shared animation
loop. Its one caller is `website-map-engine`, which draws the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map with it.

## Contents

```text
apps/website/graphics-engine/
├── Cargo.toml  the `website-graphics-engine` library; GPU dependencies on `wasm32` only
└── src/        the frame vocabulary, draw path, pipelines, shader, text, buffers and frame pump
```

## How it works

The crate is a library (`rlib`) with no binary and no features. `bytemuck` and `earcutr` are its
only dependencies on every target; `wgpu` 29 (WebGPU and WebGL backends, WGSL), `wasm-bindgen`,
`js-sys` and `web-sys` are dependencies of the `wasm32` target alone. Each source file that names
a GPU or browser type gates itself on `wasm32`, so a native build compiles the CPU half and the
native tests cover it, while the browser build adds the uploads, pipelines, encoder, present step,
compute cull and the `requestAnimationFrame` loop.

The dependency arrow runs one way. The map engine decides what to draw, in which order and with
which pipeline, and hands the result over as `FramePacket`s of `DrawBatch`es keyed by opaque lane
ids; this crate binds and draws what it is told. `src/` holds eight modules: `frame` (the
vocabulary), `draw`, `pipeline`, `shaders`, `text`, `layout` (the shared byte layouts in one
list), `device` (buffer pools) and `r#loop` (the pump).

## Getting started

Run these from the repository root:

```bash
cargo test -p website-graphics-engine --all-features  # the native unit tests of this crate
cargo xtask verify engine-layers                      # the dependency and naming walls
cargo xtask mk wasm-ci                                # fmt, clippy and tests of both engines
```

`cargo xtask mk wasm-ci` runs, in order, `cargo fmt --check`, `cargo clippy --all-targets
--all-features`, `cargo clippy --target wasm32-unknown-unknown` and `cargo test --all-features`
over `website-map-engine` and `website-graphics-engine`; `--dry-run` prints the steps. The
`wasm32` clippy step needs the `wasm32-unknown-unknown` target installed. Nothing in this crate
runs on its own: to see it draw, run the app (`cargo xtask mk leptos` with `cargo xtask mk
rust-api`) and open the Mission Creator.

## Configuration

None: the crate reads no environment variable, config file or feature flag. The browser half is
selected by the build target (`wasm32`), not by a feature. `Cargo.toml` pins edition 2024 and
Rust 1.95.

## Public surface

- `frame`: the frame vocabulary (`FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`,
  `TextRun`, the buffer types, `LaneId`, `PipelineId`, `BindGroupId`, `CameraUniform`), the cell
  atlases, `RenderDamage` and the swapchain `present` step.
- `draw`: triangulation, mesh and hairline composition, the grid, the vertex and index uploads,
  `encode::encode` and the sprite cull pair in `draw::cull`.
- `layout`: every shared byte layout, re-exported in one list.
- `pipeline`: `create_map_shader` and the ten pipeline constructors.
- `shaders::SHADER_WGSL`: the WGSL source.
- `text`: the ASCII atlas bake, glyph metrics, layout and sprite packing.
- `device::buffers`: `LanePool` and `ReadbackLane`.
- `r#loop`: `FrameTarget` and `RafPump`.

## Boundaries

- Depends on: `bytemuck` and `earcutr`; `wgpu`, `wasm-bindgen`, `js-sys` and `web-sys` on
  `wasm32`. No workspace crate.
- Used by:
  - `website-map-engine` (`apps/website/map-engine/`), the only crate that declares it, as an
    optional dependency turned on by its `world` and `render` features;
  - through the map engine, and never by a direct dependency, the frontend in
    `apps/website/frontend/` (the `world` tier natively, `render` in the browser) and the
    developer tools in `tools_v2/developer-tools/` (the `world` tier); `website-api` takes the
    map engine's `scenario` tier and does not link this crate, although `apps/website/Dockerfile`
    copies it into the image's trimmed workspace;
  - `tools_v2/xtask/`, whose `mk wasm-ci` recipe and `verify engine-layers` gate name it.
- Rules:
  - this crate never imports `website_map_engine` (`cargo xtask verify engine-layers`, rule 1);
  - no declared type, function, constant or module name contains `terrain`, `symbology`,
    `mission`, `orbat` or `arma` (rule 2);
  - inside the map engine, only `apps/website/map-engine/src/frame/mod.rs` names this crate's
    `frame` module (rule 3a), and only that file and `apps/website/map-engine/src/frame/pump.rs`
    name `device`, `pipeline`, `shaders` or `r#loop` (rule 3b);
  - the frontend never imports this crate (rule 6).
