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

/// One vehicle of the identification table; the list is ordered by name, then id.
/// @contract vehicle-database.schema.json#/definitions/Vehicle
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: String,
    pub name: String,
    pub faction: String,
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
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleWrite {
    pub name: String,
    pub faction: String,
    pub armor_type: String,
    #[serde(default)]
    pub amphibious: String,
    #[serde(default)]
    pub primary_threat: String,
    #[serde(default)]
    pub profile_image_url: String,
}

/// The `PATCH /vehicle-database/:id` body. Each outer `None` is an absent key and leaves the
/// field unchanged; for an optional field, `Some(None)` sends `null` and clears it, and
/// `Some(Some(value))` sets it.
/// @contract vehicle-database.schema.json#/definitions/VehiclePatch
#[allow(dead_code)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehiclePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_type: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "absent_null_or_value"
    )]
    pub amphibious: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "absent_null_or_value"
    )]
    pub primary_threat: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "absent_null_or_value"
    )]
    pub profile_image_url: Option<Option<String>>,
}

/// The wire form of a patch field that tells an absent key, `null` and a value apart.
///
/// `serde` reads a present `null` into an `Option<Option<T>>` as the outer `None`, the same as an
/// absent key. Reading a present key as `Some(inner)`, and leaving the absent key to
/// `#[serde(default)]`, keeps `null` as `Some(None)`; writing skips the outer `None` through
/// `skip_serializing_if` and writes the inner option as `null` or the value.
mod absent_null_or_value {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Writes a present key: `null` for `Some(None)`, the value for `Some(Some(value))`. The outer
    /// `None` is skipped before this runs, and would write `null` if it were not.
    pub fn serialize<T, S>(field: &Option<Option<T>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        match field {
            Some(inner) => inner.serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    /// Reads a present key: `null` becomes `Some(None)`, a value `Some(Some(value))`.
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Some)
    }
}

#[cfg(test)]
#[path = "tests/vehicles.rs"]
mod tests;
