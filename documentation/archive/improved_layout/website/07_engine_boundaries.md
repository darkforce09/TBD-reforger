**Status:** archived — see [the restructure program](/documentation/restructure/README.md)

# Engine boundary invariants

**Status:** Proposed design  
**Scope:** `website-graphics-engine`, `website-map-engine`, and `website-frontend`  
**Context:** TBD Reforger platform monorepo  

Detailed specification of the architectural boundaries separating GPU rendering primitives, domain
spatial computation, and frontend UI presentation.

---

## 1. Engine Layer Architecture

The website platform separates rendering concerns into three distinct tiers:

```text
┌────────────────────────────────────────────────────────┐
│               apps/website/frontend                    │
│   (Leptos / WASM SPA, UI Primitives, CAD Workspaces)   │
└──────────────────────────┬─────────────────────────────┘
                           │ consumes
                           ▼
┌────────────────────────────────────────────────────────┐
│               apps/website/map-engine                  │
│    (Spatial math, terrain streaming, mission domain)   │
└────────────┬─────────────────────────────▲─────────────┘
  calls GPU  │                             │ consumes scenario
  vocabulary │                             │ tier only
             ▼                             │
┌──────────────────────────┐  ┌────────────┴─────────────┐
│apps/website/             │  │apps/website/             │
│graphics-engine           │  │api_v2                    │
│(Pure WGPU / WGSL)        │  │(Axum, DB, SSE)           │
└──────────────────────────┘  └──────────────────────────┘
```

---

## 2. The Seven Engine Boundary Rules

Monorepo integrity is strictly guarded by `cargo xtask verify engine-layers`, which enforces the
seven rules codified in `documentation_v2/standards/engine_boundary_rules.md`:

### Rule 1: Graphics Engine Imports No Map Engine
`website-graphics-engine` must never import `website-map-engine` or mention `website_map_engine` in
its code. It is an independent, low-level WGPU graphics library.

### Rule 2: Graphics Engine Declares No Map Concepts
`website-graphics-engine` declares no items named after domain concepts: `terrain`, `symbology`,
`mission`, `orbat`, or `arma`. It operates strictly on generic graphical primitives: vertices,
indices, textures, camera matrices, pipelines, render passes, and uniform buffers.

### Rule 3a: Enumerated Packet Boundary
`website-map-engine` references `website_graphics_engine::frame` at exactly one enumerated seam:
`apps/website/map-engine/src/frame/mod.rs`. All other modules in `map-engine` reach frame primitives
through `crate::frame::...`. Direct imports of the frame vocabulary elsewhere in `map-engine` are
refused.

### Rule 3b: GPU Module Isolation
`website-map-engine` never imports GPU resource modules (`device`, `pipeline`, `shaders`, `loop`)
from `graphics-engine`, except for the two pinned residue sites documented in
`RULE3B_PIN` (`map-engine/src/frame/mod.rs` and `pump.rs`).

### Rule 4: Data Layer Isolation
`website-map-engine`'s `data/` module (which houses the pure mission format and ORBAT schemas)
never imports from `world/` (which houses spatial indexing and heightmaps). The mission data model is
mathematically decoupled from terrain rendering.

### Rule 5: Headless Editing
`website-map-engine/src/editing/` must remain completely headless: it never imports DOM or browser
types (`web_sys`, `leptos`, `wasm_bindgen`). All interactive editing commands run as pure state
machines that can be exercised by native integration tests.

### Rule 6: Frontend Never Imports Graphics Engine
`website-frontend` **never** imports `website-graphics-engine`. The frontend interacts with graphics
strictly through `website-map-engine`. The browser mounts a canvas and passes control to the map
engine's frame pump; it never configures WGPU render passes or binds shader pipelines directly.

### Rule 7: Scenario Tier Headless Purity
The `data/scenario` module of `website-map-engine` imports nothing outside itself. This allows
`website-api` to link `map-engine` in server-side environments without pulling in heavy math or
rendering crates.

---

## 3. Why Engines Sit at the Website Root

1. **Shared Headless Utility**: `map-engine` is consumed by `frontend`, `api_v2`, and
   `developer-tools`. Moving it inside `frontend/` would invert architecture by forcing the Axum
   server to depend on a frontend crate.
2. **Independent Compilation Targets**:
   - `graphics-engine` and `map-engine` compile for both `wasm32-unknown-unknown` and native host
     architectures.
   - `frontend` compiles as a WASM binary via Trunk.
   - `api_v2` compiles as a native Linux binary.
   - Siting them as peer crates allows Cargo to resolve feature flags and compilation targets cleanly.

---

## 4. Verification Commands

```bash
# 1. Run the engine layer boundary verification gate
cargo xtask verify engine-layers

# 2. Check wasm32 compilation for both engine crates
cargo clippy -p website-map-engine -p website-graphics-engine --target wasm32-unknown-unknown --all-targets -- -D warnings

# 3. Run native test suites
cargo test -p website-map-engine --all-features
cargo test -p website-graphics-engine --all-features
```
