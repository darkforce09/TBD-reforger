//! The single-prefab building-blueprint bench served at `/debug/building-viewer`.
//!
//! **Role:** a hyper-focused instrument for eyeballing what the Workbench extractor produced for
//! one building — floor plates, thickness walls, apertures, furniture cover, stairs — and for
//! driving `evaluate_los` interactively: the BVH raycast over the building's `.bvh` occlusion
//! sidecar, attributed through the blueprint, with a draggable observer and target, elevation
//! sliders, and the ray coloured along the ordered [`LosHit`] trace. Alt+click instead casts the
//! multi-floor viewshed wash — `website_map_engine::spatial::los::interior::wash::level_washes`
//! fires one BVH ray at every 0.25 m cell at eye height on every level — and the floor rail swaps
//! which level's raster the engine's `Viewshed` texture lane shows.
//!
//! **Position:** a routed workspace under `v2::apps::debug`, mounted by `app_routes.rs`, public by
//! URL and absent from the navigation. It drives the map and graphics engines directly and shares
//! none of the editor's boot machinery — no IndexedDB, no hydration, no DEM, satellite or world
//! loaders. Its interior lane tessellation lives in [`super::building_interior`].
//!
//! **Signals & state:** Leptos `RwSignal`s hold the fetched blueprint, the occlusion sidecar, the
//! assembled compound, the per-door open/closed states, the viewed floor, the observer and target,
//! the camera, and the current wash. The `RenderEngine` sits in an `Rc<RefCell<…>>` handle the
//! wasm mount owns; everything decidable is a pure function in [`geom`] — world mapping, camera
//! fit, lane tessellation, ray-span colouring, point-in-polygon — so the native test suite proves
//! the geometry with no browser and the wasm block only wires signals, listeners and the engine.
//!
//! **Invariants:** the building sits at world `ANCHOR` (6400, 6400) with blueprint +z, which is
//! game north, mapped up on screen, so f32 lane coordinates stay small; the camera is the engine's
//! own ortho camera (`zoom` = log2 px/m, capped at 6, so 64 px/m ≈ 18 px for a 0.28 m log wall).
//! Lanes are addressed by `role_id::*` constants and never by a re-copied integer. Every run is
//! reproducible from the URL: `?prefab=<map-assets path>` overrides the default farmhouse golden,
//! `?scene=1` adds exterior trees from `<slug>.scene.json`, `?doors=open` opens every leaf on load,
//! `?a=x,y,z&b=x,y,z` fixes the ray ends, and `?force=webgl` selects the headless capture backend.
//!
//! The blueprint JSON is fetched from `/map-assets/everon/prefabs/buildings/…`, served by the API
//! and proxied by Trunk in development, and the sidecar from the same path with `.json` → `.bvh`.
//! Without a sidecar the plan still draws, but LOS and the viewshed stay off and the header says
//! so. With `<slug>.instances.json` beside the blueprint the bench assembles the
//! `CompoundBuilding` — shell plus every door leaf, frame, pane and furniture BLAS under its
//! socket transform — and LOS, the wash and the section cuts all run over that instead; a click on
//! a leaf swings it.
//!
//! The 2D drawing is the mesh's own: per-floor section cuts through the collision triangles at eye
//! height (walls as true double-line outlines, windows as gaps, mullions, columns and mesh
//! furniture as outlines), plus a dim low cut for sills, the slab faces as the floor, the roof
//! faces on the Roof view, and — through a floor's voids — the floors below. The blueprint's
//! walls, plates and RoofGrid are the no-sidecar fallback; its apertures, furniture, stairs, swing
//! arcs and rings stay as annotations over the mesh. The blueprint reaches the GPU through the
//! generic upload API — `upload_polygon_mesh`, `upload_strip_tris`, `upload_hairline_segments` —
//! on these lanes:
//!
//! | lane (draw order ↑)                 | content |
//! |-------------------------------------|---------|
//! | `INTERIOR_SLABS` (poly)             | the MESH heightfield clipped below this floor's cut plane, one 0.2 m cell quad per surface, height-ramped (Roof view: the full top surface eave→ridge; fallback: blueprint plate / RoofGrid) |
//! | `INTERIOR_FURNITURE` (+outline)     | furniture / prop footprints — the compound's instances (world AABB, cover-tier colour); blueprint plates without one |
//! | `INTERIOR_WALLS` (strip)            | the MESH section cut at eye height as 0.05 m strips (fallback: blueprint walls at nominal thickness) |
//! | `INTERIOR_WALLS_OUTLINE` (hairline) | the same cut as constant 1 px hairlines, the low cut (sills), lower floors' cuts through voids, window normals, rings, ghosts |
//! | `INTERIOR_PORTALS` (+outline)       | door leaves where they hang (orange closed / green open) + door frames; outline = swing arcs (blueprint overlays / arcs without a compound) |
//! | `INTERIOR_GLAZING` (+outline)       | glass pane cuts (cyan); outline = window-frame jamb ticks |
//! | `INTERIOR_STAIRS` (hairline)        | tread hatch |
//! | `SCENE_VEGETATION` (+outline)       | `?scene=1` trees: trunk disc + canopy, rim + stipple |
//! | `Viewshed` (texture)                | the viewed level's visibility wash (`viewshed_upload`; green where A sees) |
//! | `INTERIOR_PROBE` (strip)            | the LOS ray, split + coloured at each `LosHit` (cyan past glass, yellow-green past canopy), plus event dots |
#![allow(dead_code)] // native build: the wasm host wires the live path; tests pin the pure core.

use std::sync::Arc;

use leptos::prelude::*;
use website_map_engine::spatial::bvh::sidecar::BvhSidecar;
use website_map_engine::spatial::los::interior::wash::level_wash;
use website_map_engine::spatial::los::interior::wash::level_wash_compound;
use website_map_engine::spatial::los::interior::wash::LevelWash;
use website_map_engine::spatial::los::interior::wash::WashParams;
use website_map_engine::world::architecture::blueprint::attribution_1::LosHitKind;
use website_map_engine::world::architecture::blueprint::attribution_1::LosResult;
use website_map_engine::world::architecture::blueprint::structure::BuildingBlueprint;
use website_map_engine::world::architecture::compound::assembly::CompoundBuilding;
use website_map_engine::world::architecture::section::cutter::building_drawing;
use website_map_engine::world::architecture::section::cutter::BuildingDrawing;

use super::building_interior::LevelCuts;

/// Default blueprint when no `?prefab=` override is present — the scanned FarmHouse (roof
/// heightfield + verbatim plates + attic). The hand-authored pre-scan asset stays reachable
/// via `?prefab=/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json`.
const DEFAULT_PREFAB_PATH: &str = "/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.json";

/// One end of the LOS ray in blueprint-local coordinates (x/z plan, y elevation).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayEnd {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Mirrored camera state (the engine owns the truth; these re-render the DOM overlay).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cam {
    pub tx: f64,
    pub ty: f64,
    pub zoom: f64,
}

/// What a pointer-drag currently moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drag {
    None,
    Observer,
    Target,
    Pan,
}

/// Which plan the bench displays: one architectural floor, or the roof plate above them all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewFloor {
    Level(usize),
    Roof,
}

impl ViewFloor {
    /// The elevation band this view claims, plus whether it is the topmost band (closed upper
    /// bound — the same half-open rule the raycaster's `clip_t_to_band` uses). Roof owns
    /// everything from the top level's ceiling to the building's total height.
    #[must_use]
    pub fn band(self, bp: &BuildingBlueprint) -> ([f64; 2], bool) {
        match self {
            ViewFloor::Level(i) => {
                let last = i + 1 == bp.levels.len();
                let band = bp.levels.get(i).map_or([0.0, 0.0], |l| l.elevation_range);
                // A floor below the roof band is never "topmost": the roof view owns the space
                // above the last ceiling, so every level band stays half-open.
                (band, last && bp.vertical_profile.total_height_m <= band[1])
            }
            ViewFloor::Roof => {
                let floor = bp.levels.last().map_or(0.0, |l| l.elevation_range[1]);
                ([floor, bp.vertical_profile.total_height_m.max(floor)], true)
            }
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════
// Pure geometry — native-tested, no leptos/web/engine types.
// ═════════════════════════════════════════════════════════════════════════════════════════════
pub mod geom;

mod page;
pub use page::BuildingViewerPage;

#[cfg(target_arch = "wasm32")]
mod live;
