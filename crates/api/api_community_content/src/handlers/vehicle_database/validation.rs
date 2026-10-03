//! The one validator of the vehicle database writes.
//!
//! **Role:** decodes the write bodies ([`VehicleWriteBody`] for POST and PUT, [`VehiclePatchBody`]
//! for PATCH) and checks them under one set of field rules into [`VehicleFields`], a whole row
//! ready to store, or [`VehicleFieldChanges`], the fields a PATCH changes.
//! **Position:** the create, replace and patch handlers of [`super`] decode their body into these
//! types through axum's JSON extractor and validate it before they open a transaction; the checked
//! values feed the writes of [`super::vehicle_rows`].
//! **Signals & state:** none; pure functions over the decoded body.
//! **Invariants:**
//! - Every value is trimmed before its rule applies, and a limit counts characters, as the
//!   schema's `maxLength` does: `name` at most 120, `faction` at most 60 and `armor_type` at most
//!   60 are required and never blank; `amphibious` at most 60 and `primary_threat` at most 120 are
//!   optional; `profile_image_url` is empty or a safe image URL
//!   ([`api_foundation::text::content_url_policy::is_safe_image_url`]).
//! - An empty optional value is stored as none, so an optional field is either absent or holds
//!   text.
//! - Both bodies refuse unknown keys. A PATCH key that is absent leaves its field unchanged;
//!   `null` clears an optional field and is refused on a required one.
//! - A refused body answers 400 and nothing is written.

use serde::{Deserialize, Deserializer};

use crate::models::VehicleDatabase;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::text::content_url_policy::is_safe_image_url;

/// One text field of a vehicle row: its JSON key and the most characters its trimmed value holds.
#[derive(Debug, Clone, Copy)]
struct TextField {
    key: &'static str,
    max_chars: usize,
}

/// The vehicle's display name.
const NAME: TextField = TextField {
    key: "name",
    max_chars: 120,
};
/// The side the vehicle belongs to.
const FACTION: TextField = TextField {
    key: "faction",
    max_chars: 60,
};
/// The vehicle's armour class.
const ARMOR_TYPE: TextField = TextField {
    key: "armor_type",
    max_chars: 60,
};
/// Whether and how the vehicle crosses water.
const AMPHIBIOUS: TextField = TextField {
    key: "amphibious",
    max_chars: 60,
};
/// The weapon that most endangers the vehicle.
const PRIMARY_THREAT: TextField = TextField {
    key: "primary_threat",
    max_chars: 120,
};
/// The JSON key of the vehicle's picture.
const PROFILE_IMAGE_URL_KEY: &str = "profile_image_url";

/// The POST and PUT body: every field of a vehicle row. PUT replaces all of them, so an absent
/// optional field is stored as none.
/// @contract vehicle-database.schema.json#/definitions/VehicleWrite
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleWriteBody {
    name: String,
    faction: String,
    armor_type: String,
    #[serde(default)]
    amphibious: Option<String>,
    #[serde(default)]
    primary_threat: Option<String>,
    #[serde(default)]
    profile_image_url: Option<String>,
}

/// The PATCH body. Each field is `None` when its key is absent, `Some(None)` when it is `null`
/// and `Some(Some(value))` when it holds a value.
/// @contract vehicle-database.schema.json#/definitions/VehiclePatch
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehiclePatchBody {
    #[serde(default, deserialize_with = "present_key")]
    name: Option<Option<String>>,
    #[serde(default, deserialize_with = "present_key")]
    faction: Option<Option<String>>,
    #[serde(default, deserialize_with = "present_key")]
    armor_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "present_key")]
    amphibious: Option<Option<String>>,
    #[serde(default, deserialize_with = "present_key")]
    primary_threat: Option<Option<String>>,
    #[serde(default, deserialize_with = "present_key")]
    profile_image_url: Option<Option<String>>,
}

/// Decodes a key that is present in the body: `null` becomes `Some(None)` and a value
/// `Some(Some(value))`. serde calls it only for present keys, and `#[serde(default)]` keeps an
/// absent key at `None`.
fn present_key<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

/// The checked values of a whole vehicle row, ready to store: required fields trimmed and not
/// blank, optional fields trimmed and `None` when empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VehicleFields {
    pub name: String,
    pub faction: String,
    pub armor_type: String,
    pub amphibious: Option<String>,
    pub primary_threat: Option<String>,
    pub profile_image_url: Option<String>,
}

/// The checked changes of a PATCH. `None` leaves a field unchanged; on an optional field
/// `Some(None)` clears it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VehicleFieldChanges {
    name: Option<String>,
    faction: Option<String>,
    armor_type: Option<String>,
    amphibious: Option<Option<String>>,
    primary_threat: Option<Option<String>>,
    profile_image_url: Option<Option<String>>,
}

impl VehicleWriteBody {
    /// Checks every field of the body into a whole row, or answers the first refusal as 400.
    pub fn validate(self) -> Result<VehicleFields, ApiError> {
        Ok(VehicleFields {
            name: required_text(NAME, &self.name)?,
            faction: required_text(FACTION, &self.faction)?,
            armor_type: required_text(ARMOR_TYPE, &self.armor_type)?,
            amphibious: optional_text(AMPHIBIOUS, self.amphibious.as_deref())?,
            primary_threat: optional_text(PRIMARY_THREAT, self.primary_threat.as_deref())?,
            profile_image_url: optional_image_url(self.profile_image_url.as_deref())?,
        })
    }
}

impl VehiclePatchBody {
    /// Checks every present field of the body, or answers the first refusal as 400.
    pub fn validate(self) -> Result<VehicleFieldChanges, ApiError> {
        Ok(VehicleFieldChanges {
            name: self
                .name
                .map(|v| required_patch_text(NAME, v))
                .transpose()?,
            faction: self
                .faction
                .map(|v| required_patch_text(FACTION, v))
                .transpose()?,
            armor_type: self
                .armor_type
                .map(|v| required_patch_text(ARMOR_TYPE, v))
                .transpose()?,
            amphibious: self
                .amphibious
                .map(|v| optional_text(AMPHIBIOUS, v.as_deref()))
                .transpose()?,
            primary_threat: self
                .primary_threat
                .map(|v| optional_text(PRIMARY_THREAT, v.as_deref()))
                .transpose()?,
            profile_image_url: self
                .profile_image_url
                .map(|v| optional_image_url(v.as_deref()))
                .transpose()?,
        })
    }
}

impl VehicleFieldChanges {
    /// The JSON keys this PATCH changes, in the row's field order.
    pub fn changed_keys(&self) -> Vec<&'static str> {
        [
            (NAME.key, self.name.is_some()),
            (FACTION.key, self.faction.is_some()),
            (ARMOR_TYPE.key, self.armor_type.is_some()),
            (AMPHIBIOUS.key, self.amphibious.is_some()),
            (PRIMARY_THREAT.key, self.primary_threat.is_some()),
            (PROFILE_IMAGE_URL_KEY, self.profile_image_url.is_some()),
        ]
        .into_iter()
        .filter_map(|(key, changed)| changed.then_some(key))
        .collect()
    }

    /// The whole row once these changes land on `stored`: a changed field takes its checked
    /// value and every other field keeps the stored one, which is not checked again, so a row
    /// written under older rules stays writable one field at a time.
    pub fn applied_to(self, stored: &VehicleDatabase) -> VehicleFields {
        VehicleFields {
            name: self.name.unwrap_or_else(|| stored.name.clone()),
            faction: self.faction.unwrap_or_else(|| stored.faction.clone()),
            armor_type: self.armor_type.unwrap_or_else(|| stored.armor_type.clone()),
            amphibious: self
                .amphibious
                .unwrap_or_else(|| stored_optional(&stored.amphibious)),
            primary_threat: self
                .primary_threat
                .unwrap_or_else(|| stored_optional(&stored.primary_threat)),
            profile_image_url: self
                .profile_image_url
                .unwrap_or_else(|| stored_optional(&stored.profile_image_url)),
        }
    }
}

/// A stored optional column as read through the `COALESCE(…, '')` select list: `''` is none.
fn stored_optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

/// A required field's value, trimmed; blank or over its limit answers 400.
fn required_text(field: TextField, raw: &str) -> Result<String, ApiError> {
    let value = trimmed(raw);
    if value.is_empty() {
        return Err(ApiError::bad_request(format!(
            "{} must not be blank",
            field.key
        )));
    }
    within_limit(field, value)?;
    Ok(value.to_owned())
}

/// A present PATCH value of a required field: `null` answers 400, a value is checked by
/// [`required_text`].
fn required_patch_text(field: TextField, value: Option<String>) -> Result<String, ApiError> {
    match value {
        Some(value) => required_text(field, &value),
        None => Err(ApiError::bad_request(format!(
            "{} must not be null",
            field.key
        ))),
    }
}

/// An optional field's value, trimmed: absent or empty is none; over its limit answers 400.
fn optional_text(field: TextField, raw: Option<&str>) -> Result<Option<String>, ApiError> {
    let Some(value) = raw.map(trimmed).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    within_limit(field, value)?;
    Ok(Some(value.to_owned()))
}

/// The picture URL, trimmed: absent or empty is none; anything but a safe image URL answers 400.
fn optional_image_url(raw: Option<&str>) -> Result<Option<String>, ApiError> {
    let Some(value) = raw.map(trimmed).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    if !is_safe_image_url(value) {
        return Err(ApiError::bad_request(format!(
            "{PROFILE_IMAGE_URL_KEY} must be an https:// URL or a site path such as /uploads/…"
        )));
    }
    Ok(Some(value.to_owned()))
}

/// Refuses a value longer than its field's limit, counted in characters.
fn within_limit(field: TextField, value: &str) -> Result<(), ApiError> {
    if value.chars().count() > field.max_chars {
        return Err(ApiError::bad_request(format!(
            "{} must be at most {} characters",
            field.key, field.max_chars
        )));
    }
    Ok(())
}

/// `raw` without leading and trailing whitespace, the byte order mark included, so a value the
/// schema's `\S` pattern calls blank is blank here too.
fn trimmed(raw: &str) -> &str {
    raw.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

#[cfg(test)]
#[path = "tests/validation.rs"]
mod tests;
