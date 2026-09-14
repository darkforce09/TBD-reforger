# Shared Map Engine (`src/v2/map_engine`)

This directory houses the shared WebGPU 2D rendering pipeline and tactical calculation engine.

---

## Responsibilities
- **Zero UI Opinions:** Does not dictate docks, toolbars, or buttons.
- **Rendering:** Binds to `map-engine-render` via wgpu and runs the damage-driven RAF render loop.
- **Camera:** Coordinates world-to-screen and screen-to-world projections, pan, and zoom.
- **Terrain Streaming:** Digital Elevation Model (DEM) decoding, hillshade rendering, satellite texture streaming, and forest occluder masks.
- **Tactical Algorithms:** Pure mathematical calculation modules (Ruler, LOS raycasting, Mortar ballistics solver, NATO APP-6 symbology).
