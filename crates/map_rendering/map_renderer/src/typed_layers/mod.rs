//! **Role:** the doors through which the engine lends its lanes to the typed layers it holds: the
//! symbology layers (slot symbology, glyph atlas, icon lane cull) and the world layers (buildings,
//! forest, terrain textures, terrain line of sight overlay).
//! **Position:** the map renderer; the asset sink, the frame hooks and the Mission Creator reach
//! the layers through these doors.
//! **Signals & state:** none of their own; each split borrow lends disjoint engine fields.
//! **Invariants:** a layer never receives the engine itself, only a lane sink and the parts it
//! writes, so the layer crates never name the renderer.

/// The symbology layers lent the engine's lanes, camera and text atlas.
pub(crate) mod symbology_layers;

/// The world layers lent the engine's lanes.
pub(crate) mod world_layers;
