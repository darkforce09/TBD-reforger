//! **Role:** `ModMissionDocument`: the compiled document, its wire keys, its kit substitutions and
//! its findings.
//! **Position:** `mission_compiler::game_document::document` in the `mission_compiler` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::{
    BTreeMap, Finding, KitSubstitutionReport, ModBriefing, ModEntity, ModEnvironment, ModFaction,
    ModFlow, ModMeta, ModOrbatFaction, ModRadioPlan, ModSettings, ModSlot, ModVehicle,
    ModWinConditions, ModZone, Serialize,
};

/// Canonical game-document output, including diagnostics and resource substitutions.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMissionDocument {
    /// Schema version.
    pub schema_version: String,
    /// Meta.
    pub meta: ModMeta,

    /// Environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<ModEnvironment>,
    /// Factions.
    pub factions: Vec<ModFaction>,

    /// `BTreeMap` → sorted keys, matching Go's map marshalling.
    pub orbat: BTreeMap<String, ModOrbatFaction>,
    /// Slots.
    pub slots: Vec<ModSlot>,

    /// Entities.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<ModEntity>,

    /// Radio plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_plan: Option<ModRadioPlan>,
    /// Zones.
    pub zones: Vec<ModZone>,
    /// Flow.
    pub flow: ModFlow,
    /// Win conditions.
    pub win_conditions: ModWinConditions,

    /// Empty today, and an empty carrier serialises to NOTHING — not to an empty object — so this field costs a mission that authors no optional block exactly zero bytes. That is the claim `extensions::tests::an_empty_carrier_adds_nothing_to_the_document` states in bytes and `compiler_shaped_golden_is_a_fresh_emitter_output` pins against the committed golden.
    #[serde(flatten)]
    pub extensions: mission_model::authored_blocks::ExtensionBlocks,

    /// Empty → the key is omitted entirely, which is legal (`briefings` is not in the schema's top-level `required`) and is `TBD_BriefingData.BuildOrders`' documented empty state #1. Today it is ALWAYS empty, because no mutator writes `editor.factions[].briefing` yet — see the private `derive_briefings` (in the `loadouts_briefings` module) for exactly what this reads and where the authoring gap is.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub briefings: BTreeMap<String, ModBriefing>,

    /// Settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<ModSettings>,

    /// Vehicles.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vehicles: Vec<ModVehicle>,

    /// Kit substitutions.
    #[serde(skip)]
    pub kit_substitutions: KitSubstitutionReport,

    /// Empty on a mission that authors nothing the compile discards; see the private `DiagnosticAcc` (in the `diagnostics` module) for the two corpus rules that shape it (never debug-gated; never fires on correct input).
    #[serde(skip)]
    pub diagnostics: Vec<Finding>,
}
