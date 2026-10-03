//! **Role:** The game-document compiler's module tree: the shared imports of its stages and the
//! items the crate root re-exports.
//! **Position:** `mission_compiler::game_document` in the `mission_compiler` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use crate::Error;
use mission_model::compiled::entities;
use mission_model::compiled::mission;
use mission_payload::kit_aliases::load_kit_aliases;
use mission_payload::terrain_bounds;
use mission_validation::{Finding, Primitive, Severity};
use mission_wire_safety::is_wire_unsafe;

use entities::ModEntity;
use entities::ModEntityInventory;
use entities::ModNet;
use entities::ModOrbatFaction;
use entities::ModOrbatGroup;
use entities::ModOrbatRole;
use entities::ModSlot;
use entities::ModSlotCargo;
use entities::ModSlotGear;
use entities::ModSlotLoadout;
use entities::ModVehicle;
use entities::ModVehicleSeat;
use mission::ModBriefing;
use mission::ModCircle;
use mission::ModEnvironment;
use mission::ModFaction;
use mission::ModFlow;
use mission::ModMarker;
use mission::ModMeta;
use mission::ModRadioPlan;
use mission::ModSettings;
use mission::ModWinConditions;
use mission::ModZone;
use mission::ModZoneShape;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
mod substitutions;
/// Expose substitutions ::  kit substitution at this domain boundary.
pub use substitutions::KitSubstitution;
/// Expose substitutions ::  kit substitution report at this domain boundary.
pub use substitutions::KitSubstitutionReport;
#[cfg(test)]
use substitutions::MAX_REPORTED_SUBSTITUTIONS;
use substitutions::SubstitutionAcc;
#[cfg(test)]
use substitutions::escape_resource_name;
mod document;
/// Expose document ::  mod mission document at this domain boundary.
pub use document::ModMissionDocument;
mod identity;
/// Expose identity :: compile diagnostic rule ids at this domain boundary.
pub use identity::COMPILE_DIAGNOSTIC_RULE_IDS;
/// Expose identity :: diag drop slot callsign at this domain boundary.
pub use identity::DIAG_DROP_SLOT_CALLSIGN;
/// Expose identity :: diag drop slot rank at this domain boundary.
pub use identity::DIAG_DROP_SLOT_RANK;
/// Expose identity :: diag drop slot stance at this domain boundary.
pub use identity::DIAG_DROP_SLOT_STANCE;
/// Expose identity :: diag drop slot tag at this domain boundary.
pub use identity::DIAG_DROP_SLOT_TAG;
/// Expose identity :: diag drop slot unit name at this domain boundary.
pub use identity::DIAG_DROP_SLOT_UNIT_NAME;
/// Expose identity :: diag drop squad leader at this domain boundary.
pub use identity::DIAG_DROP_SQUAD_LEADER;
/// Expose identity :: diag drop vehicle roster at this domain boundary.
pub use identity::DIAG_DROP_VEHICLE_ROSTER;
/// Expose identity :: diag win conditions at this domain boundary.
pub use identity::DIAG_WIN_CONDITIONS;
use identity::SLOT_IDENTITY_DROPS;
use identity::SLOT_RANKS;
use identity::SLOT_STANCES;
use identity::VEHICLE_SEAT_ROLES;
use identity::parse_seat_id;
mod diagnostics;
mod unsupported_authored_data;
use crate::authoring as input;
use diagnostics::DiagnosticAcc;
use diagnostics::emit_enum_identity;
use diagnostics::emit_wire_safe_identity;
use diagnostics::identity_drop_reason;
use diagnostics::is_authored;
use diagnostics::render_authored;
use diagnostics::render_authored_str;
use input::EditorPayload;
use input::EntityIn;
use input::FactionIn;
use input::SettingsIn;
use input::ShapeIn;
use input::SlotIn;
use input::SquadIn;
use input::VehicleIn;
use input::ZoneIn;
/// Expose the authored gameplay data a compiled document cannot carry.
pub use unsupported_authored_data::unsupported_authored_data;
mod type_safety;
/// Expose type safety :: scan editor payload types at this domain boundary.
pub use type_safety::scan_editor_payload_types;
mod metadata;
use metadata::CALLSIGN_FALLBACK;
use metadata::COMPILE_DATE_ANCHOR;
use metadata::META_NAME_MAX_CHARS;
use metadata::MOD_MAX_LABEL_CHARS;
use metadata::MOD_MAX_MARKER_LABEL_CHARS;
use metadata::MOD_MAX_NETS;
/// Expose metadata ::  mission meta at this domain boundary.
pub use metadata::MissionMeta;
use metadata::NET_FREQ_BASE_MHZ;
use metadata::NET_FREQ_STEP_MHZ;
use metadata::ROLE_FALLBACK;
use metadata::SPAWN_ZONE_RADIUS_M;
use metadata::mission_doc_id;
/// Expose metadata :: mission terrain key at this domain boundary.
pub use metadata::mission_terrain_key;
use metadata::or_fallback;
use metadata::slug_key;
mod loadouts_briefings;
use loadouts_briefings::derive_briefings;
use loadouts_briefings::mod_slot_loadout;
mod flow;
/// Expose flow :: flow default briefing s at this domain boundary.
pub use flow::FLOW_DEFAULT_BRIEFING_S;
/// Expose flow :: flow default jip at this domain boundary.
pub use flow::FLOW_DEFAULT_JIP;
/// Expose flow :: flow default safestart s at this domain boundary.
pub use flow::FLOW_DEFAULT_SAFESTART_S;
/// Expose flow :: flow default timelimit s at this domain boundary.
pub use flow::FLOW_DEFAULT_TIMELIMIT_S;
#[cfg(test)]
use flow::JIP_VALUES;
use flow::RadioNetSource;
use flow::cap_net_label;
use flow::derive_flow;
use flow::flow_seconds_authored;
use flow::unique_net_id;
mod win_conditions;
use win_conditions::apply_timeout_to_flow;
use win_conditions::resolve_win_conditions;
mod radio;
use radio::resolve_radio_plan;
mod vehicles;
use vehicles::derive_entities;
use vehicles::derive_vehicles_as_entities;
use vehicles::normalize_heading;
use vehicles::roster_uid;
use vehicles::vehicle_faction_key;
use vehicles::vehicle_inventory;
mod roster;
use roster::derive_settings;
use roster::derive_vehicle_roster;
mod zones;
use zones::derive_zones;
mod compile_graph;
/// Expose flatten :: flatten to mod document at this domain boundary.
pub use compile_graph::flatten_to_mod_document;
mod environment;
use environment::EnvironmentAxes;
/// Expose environment :: apply authored environment at this domain boundary.
pub use environment::apply_authored_environment;
#[cfg(test)]
use environment::clock_hhmm;
mod export;
/// Expose export ::  compiled output at this domain boundary.
pub use export::CompiledOutput;
/// Expose export :: flatten mod document json at this domain boundary.
pub use export::flatten_mod_document_json;
/// Expose export :: flatten mod document json full at this domain boundary.
pub use export::flatten_mod_document_json_full;
/// Expose export :: flatten mod document json with diagnostics at this domain boundary.
pub use export::flatten_mod_document_json_with_diagnostics;
/// Expose export :: flatten mod document json with substitutions at this domain boundary.
pub use export::flatten_mod_document_json_with_substitutions;
#[cfg(test)]
mod tests;
