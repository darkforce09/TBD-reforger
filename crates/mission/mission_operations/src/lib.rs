//! The authoring commands of the mission document.
//!
//! **Role:** every headless command the Mission Creator runs against a [`MissionDocCore`]: entity
//! placement and the ORBAT roster, faction library apply, attribute, transform and rotation edits,
//! cargo and loadouts, zones and triggers, tactical graphics, Editor Layers, compositions, document
//! search, slot identity checks, and the plain row projections the docks read.
//! **Position:** mission tier 6, over `mission_document`, `mission_crdt`, `mission_model`,
//! `mission_payload`, `mission_validation`, `orbat_slot_ids`, `formation_geometry` and
//! `map_coordinates`. The map
//! engine's hosted commands borrow the hosted document and call these with host values; the
//! Mission Creator calls them directly as well.
//! **Signals & state:** five thread-local session states (the installed cargo defaults with the
//! loadout buffer and its Apply seed, the tactical draw machine, the armed refile, the layer drag
//! and the armed rename); everything else is a pure function of the document it is handed.
//! **Invariants:** a command writes the document only through `MissionDocCore` mutators, so every
//! write is one undo step and the Yjs maps keep their wire shapes; ids cross the API as the
//! newtype ids of `mission_document` and `mission_model` and are written as their bare strings.
//!
//! [`MissionDocCore`]: mission_document::MissionDocCore

/// Apply faction-library templates to a mission.
pub mod apply_faction;
/// Resource names and authored placement payloads.
pub mod assets;
/// Attribute reads and authored-field edits.
pub mod attrs;
/// Loadout reads, copies, and authored writes.
pub mod cargo;
/// Cargo defaults and loadout JSON rules.
pub mod cargo_rules;
/// Composition capture and stored entry projections.
pub mod compositions;
/// Searchable authored entity fields.
pub mod document_index;
/// Shared document entity queries and placement bounds.
pub mod entity;
/// Authored environment values.
pub mod environment;
mod error;
/// Faction library wire values used by document operations.
pub mod faction_library;
/// Place characters into the authored faction hierarchy.
pub mod place_orbat;
pub mod prelude;
/// Ordered layer, faction, squad, and slot views of document state.
pub mod projections;
/// Faction and squad reassignment policies.
pub mod reassign;
/// Rotation snapping and world bearings.
pub mod rotation;
/// Plain authored rows consumed by document projections and host views.
pub mod rows;
/// Slot identity checks over authored squad membership.
pub mod slot_ids;
/// Tactical graphic draft and authored row edits.
pub mod tactical_graphics;
/// Authored transform calculations and document commits.
pub mod transform;
/// Zone and trigger authoring destinations.
pub mod zones;

/// Why a side-level authoring command refuses.
pub use error::Error;
/// The result of a side-level authoring command.
pub use error::Result;
