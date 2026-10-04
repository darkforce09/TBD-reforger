//! Vehicle database payloads: the identification table's rows and the administrator's writes.
//!
//! **Role:** the row `GET /vehicle-database` lists and the single-row routes answer, the full body
//! `POST` and `PUT` send, and the partial body `PATCH` sends.
//! **Position:** deserialised straight from the backend's JSON and handed to the doctrine vehicle
//! pages; the write bodies are serialised for the administrator's routes; re-serialised unchanged
//! by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** a row's empty `amphibious`, `primary_threat` and `profile_image_url` are absent
//! on the wire, never empty strings. A patch tells three states of each optional field apart: an
//! absent key leaves the field as stored, `null` clears it, and a value sets it; a required field
//! is absent or a value, never `null`, because the backend refuses a cleared required field.
//! @contract vehicle-database.schema.json#/definitions/Vehicle
//! @contract vehicle-database.schema.json#/definitions/VehiclePatch

use serde::{Deserialize, Serialize};

#[cfg(test)]
use super::common::absent_null_or_value;
use super::identifiers::VehicleDatabaseId;

/// One vehicle of the identification table; the list is ordered by name, then id.
/// @contract vehicle-database.schema.json#/definitions/Vehicle
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vehicle {
    /// The vehicle row's key.
    pub id: VehicleDatabaseId,
    /// The vehicle's name.
    pub name: String,
    /// The faction that fields it.
    pub faction: String,
    /// Its armour class.
    pub armor_type: String,
    /// Empty when the row records none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub amphibious: String,
    /// Empty when the row records none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub primary_threat: String,
    /// Empty when the row has no image.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub profile_image_url: String,
}

/// The `POST /vehicle-database` and `PUT /vehicle-database/:id` body. `PUT` replaces every field,
/// and an empty optional field is stored as none. The backend trims each value before its limit:
/// `name` and `primary_threat` 120 characters, the others 60, and `profile_image_url` empty, an
/// `https://` URL or a site path `/…`.
/// @contract vehicle-database.schema.json#/definitions/VehicleWrite
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleWrite {
    /// The vehicle's name.
    pub name: String,
    /// The faction that fields it.
    pub faction: String,
    /// Its armour class.
    pub armor_type: String,
    /// Whether and how it crosses water; omitted on the wire when empty.
    #[serde(default)]
    pub amphibious: String,
    /// The threat it poses first; omitted on the wire when empty.
    #[serde(default)]
    pub primary_threat: String,
    /// The profile image URL; omitted on the wire when empty.
    #[serde(default)]
    pub profile_image_url: String,
}

/// The `PATCH /vehicle-database/:id` body. Each outer `None` is an absent key and leaves the
/// field unchanged; for an optional field, `Some(None)` sends `null` and clears it, and
/// `Some(Some(value))` sets it.
/// @contract vehicle-database.schema.json#/definitions/VehiclePatch
#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehiclePatch {
    /// The vehicle's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The faction that fields it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    /// Its armour class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_type: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "absent_null_or_value"
    )]
    /// Whether and how it crosses water; omitted on the wire when empty.
    pub amphibious: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "absent_null_or_value"
    )]
    /// The threat it poses first; omitted on the wire when empty.
    pub primary_threat: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "absent_null_or_value"
    )]
    /// The profile image URL; omitted on the wire when empty.
    pub profile_image_url: Option<Option<String>>,
}

#[cfg(test)]
#[path = "tests/vehicles.rs"]
mod tests;
