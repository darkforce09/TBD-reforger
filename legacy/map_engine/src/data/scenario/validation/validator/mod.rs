//! Role: Module boundary for mission/validation/validator.
//! Position: `mission/validation/validator` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::data::scenario::compile::terrain_bounds;
use serde_json::Value;
use std::collections::HashSet;
mod context;
/// Expose context ::  eval context at this domain boundary.
pub use context::EvalContext;
/// Expose context ::  finding at this domain boundary.
pub use context::Finding;
/// Expose context ::  loadout policy at this domain boundary.
pub use context::LoadoutPolicy;
/// Expose context ::  primitive at this domain boundary.
pub use context::Primitive;
/// Expose context ::  severity at this domain boundary.
pub use context::Severity;
mod registry;
/// Expose registry ::  registry at this domain boundary.
pub use registry::Registry;
/// Expose registry ::  rule at this domain boundary.
pub use registry::Rule;
/// Expose registry ::  self check failure at this domain boundary.
pub use registry::SelfCheckFailure;
mod rules;
/// Expose rules :: default registry at this domain boundary.
pub use rules::default_registry;
use rules::editor_factions;
use rules::editor_slots;
use rules::editor_squads;
use rules::no_trip_context;
use rules::slot_id;
use rules::squad_id;
use rules::str_array;
use rules::str_field;
use rules::terrain_key;
use rules::top_level_array;
/// Expose rules :: validate editor payload at this domain boundary.
pub use rules::validate_editor_payload;
mod scenario;
use scenario::rule_v1_player_spawn;
use scenario::rule_v2_faction_max;
use scenario::rule_v3_slot_in_bounds;
use scenario::rule_v4_schema_version;
mod orbat;
use orbat::rule_orbat_callsign_unique;
use orbat::rule_orbat_identity_filled;
use orbat::rule_orbat_slot_resolves;
use orbat::rule_orbat_squad_has_leader;
use orbat::rule_orbat_template_coverage;
mod assets;
use assets::rule_asset_resolves;
mod loadout;
use loadout::declares_loadout;
use loadout::loadout_of;
use loadout::rule_loadout_has_uniform;
use loadout::rule_loadout_has_vest;
use loadout::rule_loadout_mag_count;
mod cargo;
use cargo::rule_cargo_over_capacity;
use cargo::rule_loadout_has_equipment;
use cargo::rule_vehicle_cargo_policy;
#[cfg(test)]
mod tests;
