# Developer Tools (`tools_v2/developer-tools`)

## Phase-two implementation

This is the live heavy-tooling package, imported as `developer_tools`. Executables retain their names: `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`.

- `src/blueprint`: voxel interpretation, mesh extraction, BVH compilation, ingestion, and parity reporting.
- `src/enfusion_pak`: shared FORM/PAC1 parser, payload reader, and explicit world/blueprint filesystem policies.
- `src/map_verification`: object goldens, labels, terrain manifests, BLAS manifests, and world line-of-sight checks.
- `test_fixtures/blueprint`: byte-preserved blueprint, prefab, and world-parity fixtures.
- Browser, map, world, and Enfusion tooling retain their existing module layout and fixtures.

Map command entry points accept the active repository path and return the existing exit-code result. The crate does not depend on `xtask`. The structure below is the later decomposition target, not a description of every live directory.

---

## 1. Directory Structure & Subsystems

```text
tools_v2/developer-tools/
├── Cargo.toml                           <-- tokio, axum, image, bcdec_rs, website-map-engine
├── src/
│   ├── lib.rs                           <-- Library root
│   ├── bin/                             <-- 6 dedicated binary entrypoints (<250 LOC each)
│   ├── browser_testing/                 <-- Headless Chrome DevTools Protocol (CDP) test harness
│   ├── enfusion_tooling/                <-- Script indexer, symbol scanner, apidoc scraper
│   ├── enfusion_pak/                    <-- UNIFIED .pak archive reader & VFS (deduplicated)
│   ├── blueprint/                       <-- (Relocated from xtask/src/map_blueprint) 3D mesh & BVH
│   ├── map_raster_pipeline/             <-- Aerial orthophoto, satellite pyramids, cartography
│   └── world_export_pipeline/           <-- 512m chunking, DEM elevation, road graphs, gates
└── tests/                               <-- Extracted test suites
```

---

## 2. Binary Entrypoints (`src/bin/`)

| Binary | Previous Name | Responsibility |
|---|---|---|
| **`browser_test_runner`** | `src/bin/gate.rs` | Headless Chrome DevTools Protocol (CDP) test harness for WebGL2/WebGPU editor routes. |
| **`screen_capture`** | `src/bin/capture.rs` | Headless editor canvas snapshot and zoom sweep utility. |
| **`enfusion_oracle`** | `src/bin/enf.rs` | Unpacks game scripts, indexes symbols, extracts class members from Doxygen. |
| **`mcp_broker`** | `src/bin/mcpd.rs` | Persistent AF_UNIX socket broker serializing requests to Enfusion NetAPI. |
| **`map_pipeline`** | `src/bin/map.rs` | 2D aerial orthophoto stitching, water mask tinting, satellite tile pyramids. |
| **`world_pipeline`** | `src/bin/world.rs` | Macro world export: 512m chunk partitioning, road network graphs, DEM processing. |

---

## 3. Subsystem Descriptions

- **`browser_testing/`**: Drives the headless Chromium instance over WebSockets via CDP. Spawns the static Leptos SPA server and API reverse proxy, executes DOM regression tests (`dom_oracle/`, formerly `vsuite.rs`), verifies route drift (`route_drift/`, formerly `sroutes.rs`), and runs the 21-scenario editor smoke test suite.
- **`enfusion_pak/`**: Consolidated, high-performance Enfusion `.pak` virtual filesystem reader. Unifies the previously duplicated readers in `tbd-tools/src/world/pak.rs` and `xtask/src/map_blueprint/pak.rs`.
- **`blueprint/`**: Relocated from `xtask/src/map_blueprint/` (13,409 LOC, 32 files). Decodes binary `.xob` 3D model meshes, raymarches triangles into voxel occupancy grids, detects walls, floor slabs, and plates, and compiles 3D BVH collision acceleration trees.
- **`map_raster_pipeline/`**: Renames cryptic legacy modules (`sap.rs` → `aerial_orthophoto/`, `tbds_v2.rs` → `satellite_container/`, `water.rs` → `inland_water/`, `labels.rs` → `map_labels/`).
- **`world_export_pipeline/`**: Decomposes legacy monoliths (`aux.rs` 1,644 LOC, `build.rs` 1,407 LOC, `gates.rs` 1,290 LOC, `forest_smooth.rs` 1,244 LOC) into focused single-responsibility modules <500 LOC.
