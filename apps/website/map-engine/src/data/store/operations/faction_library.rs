//! Role: faction library.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One role a faction fields, with the loadout attached to it.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FactionRole {
    /// Role.
    pub role: String,

    /// Tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,

    /// Character.
    pub character: String,

    /// The loadout document for this role, carried opaquely — the editor owns its shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loadout: Option<Value>,
}

/// One vehicle a faction fields.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FactionVehicle {
    /// Vehicle.
    pub vehicle: String,

    /// Label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// A faction in full: its roles, its vehicles, and the doctrine attached to it.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FactionDoc {
    /// Side.
    pub side: String,

    /// Name.
    pub name: String,

    /// Emblem.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emblem: Option<String>,

    /// Roles.
    #[serde(default)]
    pub roles: Vec<FactionRole>,

    /// Vehicles.
    #[serde(default)]
    pub vehicles: Vec<FactionVehicle>,
}
