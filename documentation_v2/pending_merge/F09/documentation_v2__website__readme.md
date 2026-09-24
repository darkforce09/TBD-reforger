# Website Platform Suite (`website/`)

Documentation for the web platform applications and rendering engines, mirroring `apps/website/`.

## Subsystem Architecture
- `api/`: Rust Axum REST API and Server-Sent Events (SSE) realtime broadcast hub (:8080).
- `graphics_engine/`: Pure WebGPU GPU rendering primitives (pipelines, shaders, draw batching). Knows zero map concepts.
- `map_engine/`: Map graphics, spatial 3D BVH, terrain DEM streaming, camera math, and Yjs CRDT scenario store.
- `frontend/`: Leptos 0.8 CSR single-page application (Trunk/WASM, :3000) housing CAD workspaces, standard navigation, and design system primitives.

## Code Mapping
- `apps/website/api_v2/`
- `apps/website/graphics-engine/`
- `apps/website/map-engine/`
- `apps/website/frontend/`
