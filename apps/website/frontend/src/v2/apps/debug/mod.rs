//! The engine diagnostics testbenches.
//!
//! **Role:** groups the URL-only benches that drive one engine subsystem in isolation, with none
//! of the editor's document, persistence or chrome around it — the building-blueprint viewer at
//! `/debug/building-viewer`, which loads a single extracted prefab and probes line of sight and
//! the viewshed through it, and the world-occluder bench at `/debug/world-los`, which loads the
//! committed object catalogue around a map point and probes one segment through it.
//! **Position:** a workspace under `v2::apps`, routed by `app_routes.rs`. Each bench mounts its own
//! canvas and speaks to the map and graphics engines directly; none of them is reachable from the
//! navigation, and nothing in the platform reaches back into them.
//! **Signals & state:** none at this level. Each bench owns its own signals and engine handle.
//! **Invariants:** a bench reads committed assets and engine code only — it never writes a mission
//! document, never persists, and never imports from a sibling workspace. Every parameter of a run
//! is in the URL, so a reading reproduces exactly.

/// Interior plan lanes shared by both benches: walls, doors, glazing, furniture and vegetation.
pub mod building_interior;
/// The single-prefab blueprint bench behind `/debug/building-viewer`.
pub mod building_viewer;
/// The world-occluder bench behind `/debug/world-los`.
pub mod world_los;
/// Plan geometry for the world-occluder bench: footprints, section cuts and the probe ray.
pub mod world_los_scene;
