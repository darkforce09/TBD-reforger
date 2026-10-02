# Graphics engine

The `graphics_engine` crate: the web platform's WebGPU renderer, which knows no map
concept. It defines the GPU half of the frame vocabulary a caller describes a frame in, and
supplies the render pipelines, the vertex and index uploads, the frame encoder, the GPU sprite cull
and the shared animation loop. The GPU-free half it builds on (instance layouts, geometry, glyph
packing, frame ids, damage tracking and the WGSL source) is the
`crates/graphics/render_primitives/` crate. Its one caller is `map_engine`, which draws the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map with it.

## Contents

```text
legacy/graphics_engine/
├── Cargo.toml  the `graphics_engine` library; GPU dependencies on `wasm32` only
└── src/        the GPU frame vocabulary, draw path, pipelines, buffers and frame pump
```

## How it works

The crate is a library (`rlib`) with no binary and no features. `bytemuck` and the workspace crate
`render_primitives` are its only dependencies on every target; `wgpu` 29 (WebGPU and WebGL backends, WGSL), `wasm-bindgen`
and `web-sys` are dependencies of the `wasm32` target alone. Each source file that names
a GPU or browser type gates itself on `wasm32`, so a native build compiles the CPU half and the
native tests cover it, while the browser build adds the uploads, pipelines, encoder, present step,
compute cull and the `requestAnimationFrame` loop.

The dependency arrow runs one way. The map engine decides what to draw, in which order and with
which pipeline, and hands the result over as `FramePacket`s of `DrawBatch`es keyed by opaque lane
ids; this crate binds and draws what it is told. `src/` holds five modules: `frame` (the
vocabulary), `draw`, `pipeline`, `device` (buffer pools) and `r#loop` (the pump). Callers import
the GPU-free half (instance layouts, geometry, mesh composition, glyph text, frame ids, the camera
uniform and damage tracking) from `render_primitives` directly; this crate forwards none of it.

## Getting started

Run these from the repository root:

```bash
cargo test -p graphics_engine --all-features  # the native unit tests of this crate
cargo xtask verify engine-layers                      # the dependency and naming walls
cargo xtask mk wasm-ci                                # fmt, clippy and tests of both engines
```

`cargo xtask mk wasm-ci` runs, in order, `cargo fmt --check`, `cargo clippy --all-targets
--all-features`, `cargo clippy --target wasm32-unknown-unknown` and `cargo test --all-features`
over `map_engine` and `graphics_engine`; `--dry-run` prints the steps. The
`wasm32` clippy step needs the `wasm32-unknown-unknown` target installed. Nothing in this crate
runs on its own: to see it draw, run the app (`cargo xtask mk leptos` with `cargo xtask mk
rust-api`) and open the Mission Creator.

## Configuration

None: the crate reads no environment variable, config file or feature flag. The browser half is
selected by the build target (`wasm32`), not by a feature. `Cargo.toml` pins edition 2024 and
Rust 1.95.

## Public surface

- `frame`: the GPU frame vocabulary (`FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`,
  `TextRun`, the buffer types), the cell atlases and the swapchain `present` step.
- `draw`: the vertex and index uploads, `encode::encode` and the compute cull in `draw::cull`.
- `pipeline`: `create_map_shader` and the ten pipeline constructors.
- `device::buffers`: `LanePool` and `ReadbackLane`.
- `r#loop`: `FrameTarget` and `RafPump`.

## Boundaries

- Depends on: `bytemuck` and `render_primitives`; `wgpu`, `wasm-bindgen` and `web-sys` on
  `wasm32`.
- Used by:
  - `map_engine` (`legacy/map_engine/`), the only crate that declares it, as an
    optional dependency turned on by its `world` and `render` features;
  - through the map engine, and never by a direct dependency, the frontend in
    `apps/frontend/` (the `world` tier natively, `render` in the browser) and the
    developer tools in `tools/developer_tools/` (the `world` tier); `api` takes the
    map engine's `scenario` tier and does not link this crate, although `deploy/Dockerfile`
    copies it into the image's trimmed workspace;
  - `tools/xtask/`, whose `mk wasm-ci` recipe and `verify engine-layers` gate name it.
- Rules:
  - this crate never imports `map_engine` (`cargo xtask verify engine-layers`, rule 1);
  - no declared type, function, constant or module name contains `terrain`, `symbology`,
    `mission`, `orbat` or `arma` (rule 2);
  - inside the map engine, only `legacy/map_engine/src/frame/mod.rs` names this crate's
    `frame` module (rule 3a), and only that file and `legacy/map_engine/src/frame/pump.rs`
    name `device`, `pipeline`, `shaders` or `r#loop` (rule 3b);
  - the frontend never imports this crate (rule 6).

## Related documentation

- [Graphics engine overview](/documentation/legacy/graphics_engine/graphics_engine_overview.md)
  — one frame across the modules, the design and the open work.
- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the one-way
  arrow and the rules `cargo xtask verify engine-layers` holds this crate to.
