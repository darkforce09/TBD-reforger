//! Role: library.
//! Position: `doc/operations/apply_faction` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;
use super::Value;
use super::plural;

/// Everon map centre (`12800² / 2`) — matches Leptos `INITIAL_TARGET`, and the Apply anchor for every terrain whose bounds are Everon's (`everon`, `custom`, anything unknown).
pub const APPLY_ANCHOR_X: f64 = 6400.0;

/// Canonical apply anchor y value.
pub const APPLY_ANCHOR_Y: f64 = 6400.0;

/// Canonical arland anchor x value.
pub(super) const ARLAND_ANCHOR_X: f64 = 2048.0;

/// Canonical arland anchor y value.
pub(super) const ARLAND_ANCHOR_Y: f64 = 2048.0;

/// Canonical slot spacing x value.
pub(super) const SLOT_SPACING_X: f64 = 15.0;

/// Apply anchor for terrain using the supplied domain data.
pub(super) fn apply_anchor_for_terrain(terrain: &str) -> (f64, f64) {
    match terrain {
        "arland" => (ARLAND_ANCHOR_X, ARLAND_ANCHOR_Y),

        _ => (APPLY_ANCHOR_X, APPLY_ANCHOR_Y),
    }
}

/// Apply anchor xy using the supplied domain data.
pub(super) fn apply_anchor_xy(doc: &MissionDocCore) -> (f64, f64) {
    let terrain = serde_json::from_str::<Value>(&doc.small_maps_json())
        .ok()
        .and_then(|root| {
            root.get("meta")
                .and_then(|m| m.get("terrain"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| "everon".to_string());
    apply_anchor_for_terrain(&terrain)
}

/// Canonical valid sides value.
pub(super) const VALID_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

/// One role row from `faction-library.schema.json` (serde-owned; no FE types).
#[derive(Debug, Clone)]
pub struct FactionLibraryRole {
    /// Role.
    pub role: String,

    /// Tag.
    pub tag: Option<String>,

    /// Character.
    pub character: String,

    /// Loadout.
    pub loadout: Option<Value>,
}

/// One vehicle row from the library pool.
#[derive(Debug, Clone)]
pub struct FactionLibraryVehicle {
    /// Vehicle.
    pub vehicle: String,

    /// Label.
    pub label: Option<String>,
}

/// Library payload for [`apply_faction_library`] (name + roles + vehicles).
#[derive(Debug, Clone)]
pub struct FactionLibraryInput {
    /// Name.
    pub name: String,

    /// Roles.
    pub roles: Vec<FactionLibraryRole>,

    /// Vehicles.
    pub vehicles: Vec<FactionLibraryVehicle>,
}

/// Result of a successful Apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyFactionResult {
    /// Faction id.
    pub faction_id: String,

    /// Squad id.
    pub squad_id: String,

    /// Leader slot id.
    pub leader_slot_id: String,

    /// Roles applied.
    pub roles_applied: usize,

    /// Vehicles applied.
    pub vehicles_applied: usize,
}

/// Domain representation of authored squad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredSquad {
    /// The squad's id.
    pub id: String,

    /// The label the operator sees in the ORBAT tree (falls back to [`Self::id`] when unnamed).
    pub name: String,

    /// Why it reads as authored, phrased for the operator: `"3 slots"`, `"renamed"`, ….
    pub why: String,

    /// Slots it holds — the bodies that would change hands.
    pub slots: usize,
}

/// Error from [`apply_faction_library`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyFactionError {
    /// `side` was not BLUFOR / OPFOR / INDFOR.
    InvalidSide(String),

    /// Domain representation of would collapse squads.
    WouldCollapseSquads {
        side: String,

        squad_ids: Vec<String>,

        blocking: Vec<AuthoredSquad>,

        slots_at_risk: usize,
    },
}

impl std::fmt::Display for ApplyFactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSide(s) => write!(f, "invalid side {s:?}; expected BLUFOR|OPFOR|INDFOR"),
            Self::WouldCollapseSquads {
                side,
                squad_ids,
                blocking,
                slots_at_risk,
            } => {
                let named = blocking
                    .iter()
                    .map(|b| format!("\"{}\" ({})", b.name, b.why))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    f,
                    "refusing to apply a template onto {side}: it has {} squads, {} of them \
                     holding ORBAT you authored — {named} — and a faction template is a flat role \
                     list with no squad level. Applying folds the whole side into one squad, so \
                     those squad boundaries, names, callsigns and vehicles would be gone for good: \
                     Save-as-template never captured them, so nothing can put them back. Nothing \
                     was changed — those squads, and the {} in them, are exactly as you left them, \
                     leaders, callsigns, ranks and map positions included. Merge them into one \
                     squad or delete them, then apply again. (Squads a map placement created and \
                     you never edited are folded in automatically; these are not those.)",
                    squad_ids.len(),
                    blocking.len(),
                    plural(*slots_at_risk, "slot"),
                )
            }
        }
    }
}

impl std::error::Error for ApplyFactionError {}
