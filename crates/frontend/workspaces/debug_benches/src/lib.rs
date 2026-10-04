//! The engine diagnostics benches of the single-page app.
//!
//! **Role:** the URL-only benches that drive one engine subsystem in isolation, with none of the
//! Mission Creator's document, persistence or chrome around it: the building-blueprint viewer at
//! `/debug/building-viewer`, which loads a single extracted prefab and probes line of sight and
//! the viewshed through it; the world-occluder bench at `/debug/world-los`, which loads the
//! committed object catalogue around a map point and probes one segment through it; the equipment
//! data viewer at `/debug/data-viewer`; and the ballistics agreement bench at
//! `/debug/ballistics-agreement`.
//! **Position:** a workspace crate the app's `app_routes.rs` mounts by its four route components.
//! Each bench mounts its own canvas and speaks to the map and graphics engines directly; none of
//! them is reachable from the navigation, and nothing in the platform reaches back into them.
//! **Signals & state:** none at this level. Each bench owns its own signals and engine handle.
//! The route components and their browser hosts compile for `wasm32` only; the pure halves
//! (interior lanes, scene geometry, the building viewer's geometry, the agreement reading, the
//! data viewer's URL and browsing state) compile on every target, so their tests run natively.
//! **Invariants:** a bench reads committed assets, engine code and anonymous API reads only — it
//! never writes a mission document, never persists, and never depends on a page or another
//! workspace. Every parameter of a run is in the URL, so a reading reproduces exactly.

/// The native/wasm ballistics agreement bench behind `/debug/ballistics-agreement`.
pub mod ballistics_agreement;
/// Interior plan lanes shared by both map benches: walls, doors, glazing, furniture and
/// vegetation.
pub mod building_interior;
/// The single-prefab blueprint bench behind `/debug/building-viewer`.
pub mod building_viewer;
/// Published equipment and vehicle source inspector behind `/debug/data-viewer`.
pub mod data_viewer;
/// Why a bench run cannot start, and the crate's result type.
pub mod error;
/// The crate's most-used items: the four route components and the shared interior lanes.
pub mod prelude;
/// The world-occluder bench behind `/debug/world-los`.
pub mod world_los;
/// Plan geometry for the world-occluder bench: footprints, section cuts and the probe ray.
pub mod world_los_scene;

pub use error::{Error, Result};
