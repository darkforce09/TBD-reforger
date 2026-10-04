//! The Mission Creator's pure state vocabulary: the values, tables and registered cells every
//! other editor layer reads.
//!
//! **Role:** owns the editor state that needs no engine handle, no browser session and no rendered
//! surface to be decided: the chrome insets and class recipes, the review mode predicate, the
//! world-layer preferences, the asset catalog and the Arsenal's loadout rules, the outliner node
//! model, the zone vocabulary and geometry, the marker icon tables, the map scale arithmetic, the
//! transform-widget and armed-place vocabularies, the seam registration every mounted hook uses,
//! and the recently-placed recorder cell.
//! **Position:** the lowest Mission Creator crate. It depends on the foundation crates, the
//! mission, editing and streaming crates, `leptos` and `serde`; the engine bridge, the session, the
//! Arsenal and the workspace crates read it, and it reads none of them.
//! **Signals & state:** the chrome inset cells, the review workspace cell and the
//! recently-placed recorder cell are thread-local and tab-scoped; everything else is pure data
//! and functions.
//! **Invariants:** depends on no Mission Creator crate; a registered cell is written by its
//! upper-layer owner at mount and cleared by that owner at unmount; plain library code compiles
//! on every target, and only the `localStorage` reads and writes are `wasm32`-only.

/// The armed placement's pointer-up decision: place, keep armed, fall through to a pan, disarm.
pub mod armed_place;
/// The Arsenal's domain rules: the loadout rows, the compatibility graph, per-row option
/// building, validation, the doll region model and the weight readout.
pub mod arsenal_rules;
/// Flat registry rows turned into the palette's faction, vehicle and object trees, with the
/// catalog search and its bounded pattern language.
pub mod asset_catalog;
/// The crate's error: why a loadout document is refused against the shipped export schema.
pub mod error;
/// The keys the catalog and outliner trees name their nodes by.
pub mod ids;
/// The chrome inset constants and cells, the status bar height, and the shared class recipes the
/// strip, docks and toolbelt are laid out from.
pub mod layout;
/// The closed marker icon vocabulary from the mission schema and the canonical picker rows it
/// folds into.
pub mod marker_icons;
/// The Editor Layers outliner's node model: the folder tree, the ORBAT tree, the flattened rows
/// the windowed renderer draws, and the folder slot membership.
pub mod outliner_model;
/// The items most callers name, for `use mission_creator_state::prelude::*;`.
pub mod prelude;
/// The recently-placed recorder cell the right dock registers and placements outside the dock
/// record into.
pub mod recent_placements;
/// The read-only review workspace: the reviewed version an editor mount shows, and the one
/// predicate every write path consults before writing the mission.
pub mod review_mode;
/// Metres per screen pixel, the scale bar choice and its readout.
pub mod scale_math;
/// The registration every mounted hook uses: install now, clear at the owner's unmount, and
/// clear only the registration that owner installed.
pub mod seam_registration;
/// The transform widget's variant and snap vocabulary.
pub mod transform;
/// Per-user world-layer visibility and basemap preferences, persisted to local storage.
pub mod world_layer_prefs;
/// The zone vocabulary read from the mission schema and the zone geometry the draw tool commits.
pub mod zones;
