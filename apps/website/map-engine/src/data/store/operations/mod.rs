//! Role: Module boundary for doc/operations.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Apply faction-library templates to a mission.
pub mod apply_faction;

/// Place characters into the authored faction hierarchy.
pub mod place_orbat;

/// Attribute reads and authored-field edits.
pub mod attrs;

/// Composition capture and stored entry projections.
pub mod compositions;

/// Shared document entity queries and placement bounds.
pub mod entity;

/// Plain authored rows consumed by document projections and host views.
pub mod rows;

/// Ordered layer, faction, squad, and slot views of document state.
pub mod projections;

/// Placement patterns over explicit world positions.
pub mod placement;

/// Rotation snapping and world bearings.
pub mod rotation;

/// Authored transform calculations and document commits.
pub mod transform;

/// Faction and squad reassignment policies.
pub mod reassign;

/// Faction library wire values used by document operations.
pub mod faction_library;

/// Resource names and authored placement payloads.
pub mod assets;

/// Cargo defaults and loadout JSON rules.
pub mod cargo_rules;

/// Loadout reads, copies, and authored writes.
pub mod cargo;

/// Authored environment values.
pub mod environment;

/// Zone and trigger authoring destinations.
pub mod zones;

/// Tactical graphic draft and authored row edits.
pub mod tactical_graphics;

/// Searchable authored entity fields.
pub mod document_index;

/// Slot identity checks over authored squad membership.
pub mod slot_ids;
