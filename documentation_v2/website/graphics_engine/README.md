# Pure GPU Graphics Engine (`website/graphics_engine/`)

The graphics engine is a pure WebGPU hardware rendering primitive library built on `wgpu`.

## Strict Boundary Law
The graphics engine knows **zero map concepts**. It has no understanding of coordinates, latitude/longitude, MGRS, terrain elevations, waypoints, or military units. It accepts uniform buffers, vertex/index buffers, and instanced draw command packets.

## Subsystems
- `gpu_context/`: Adapter selection, logical device acquisition, queue lifecycle, surface configuration.
- `pipeline/`: Render pipelines, bind group layouts, depth/stencil states, multisampling.
- `render_passes/`: Geometry rendering passes, wireframes, instanced batching, SDF glyph rendering.
- `frame_loop/`: Damage-driven `requestAnimationFrame` render loop executing redraws only upon state invalidation.
- `shaders/`: WGSL shader source code modules.

## Code Mapping
- Source: `apps/website/graphics-engine/`
