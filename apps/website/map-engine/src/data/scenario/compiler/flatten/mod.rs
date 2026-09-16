//! Role: Module boundary for mission/compiler/flatten.
//! Position: `mission/compiler/flatten` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use crate::data::scenario::ast::entities;
use crate::data::scenario::ast::scenario;
use crate::data::scenario::compile::terrain_bounds;
use crate::data::scenario::kit::load_kit_aliases;
use crate::data::scenario::validate::{Finding, Primitive, Severity};
use crate::data::scenario::wire_safety::is_wire_unsafe;
/// Expose entities ::  mod entity at this domain boundary.
pub use entities::ModEntity;
/// Expose entities ::  mod entity inventory at this domain boundary.
pub use entities::ModEntityInventory;
/// Expose entities ::  mod net at this domain boundary.
pub use entities::ModNet;
/// Expose entities ::  mod orbat faction at this domain boundary.
pub use entities::ModOrbatFaction;
/// Expose entities ::  mod orbat group at this domain boundary.
pub use entities::ModOrbatGroup;
/// Expose entities ::  mod orbat role at this domain boundary.
pub use entities::ModOrbatRole;
/// Expose entities ::  mod slot at this domain boundary.
pub use entities::ModSlot;
/// Expose entities ::  mod slot cargo at this domain boundary.
pub use entities::ModSlotCargo;
/// Expose entities ::  mod slot gear at this domain boundary.
pub use entities::ModSlotGear;
/// Expose entities ::  mod slot loadout at this domain boundary.
pub use entities::ModSlotLoadout;
/// Expose entities ::  mod vehicle at this domain boundary.
pub use entities::ModVehicle;
/// Expose entities ::  mod vehicle seat at this domain boundary.
pub use entities::ModVehicleSeat;
/// Expose scenario ::  mod briefing at this domain boundary.
pub use scenario::ModBriefing;
/// Expose scenario ::  mod circle at this domain boundary.
pub use scenario::ModCircle;
/// Expose scenario ::  mod environment at this domain boundary.
pub use scenario::ModEnvironment;
/// Expose scenario ::  mod faction at this domain boundary.
pub use scenario::ModFaction;
/// Expose scenario ::  mod flow at this domain boundary.
pub use scenario::ModFlow;
/// Expose scenario ::  mod marker at this domain boundary.
pub use scenario::ModMarker;
/// Expose scenario ::  mod meta at this domain boundary.
pub use scenario::ModMeta;
/// Expose scenario ::  mod radio plan at this domain boundary.
pub use scenario::ModRadioPlan;
/// Expose scenario ::  mod settings at this domain boundary.
pub use scenario::ModSettings;
/// Expose scenario ::  mod win conditions at this domain boundary.
pub use scenario::ModWinConditions;
/// Expose scenario ::  mod zone at this domain boundary.
pub use scenario::ModZone;
/// Expose scenario ::  mod zone shape at this domain boundary.
pub use scenario::ModZoneShape;
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
/// Expose document ::  compile error at this domain boundary.
pub use document::CompileError;
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
use crate::data::scenario::ast::authoring as input;
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
