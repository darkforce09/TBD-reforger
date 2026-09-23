// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/faction-library.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{Role, Vehicle};

///One operator-authored reusable faction: a side and display name plus its ORBAT role templates (each wrapping a registry character with an optional SlotLoadout v2) and its vehicle pool. Stored as the jsonb document of a `user_factions` row; the Mission Creator palette renders side → faction → roles/vehicles from these rather than from the raw vanilla registry dump. Role loadouts reuse the loadout-export v2 shapes: `wear` is an open map keyed by engine LoadoutSlotInfo name, and weapons are slot-indexed.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TbdFactionLibraryEntry {
    ///Optional emblem asset path/URL (UI later).
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub emblem: ::std::option::Option<::std::string::String>,
    ///Display name, e.g. 'US Army 1980s'.
    pub name: TbdFactionLibraryEntryName,
    ///ORBAT role templates in authored order — the palette's draggable leaves.
    pub roles: ::std::vec::Vec<Role>,
    ///Export-side key (mirrors the mission doc Faction.key vocabulary).
    pub side: TbdFactionLibraryEntrySide,
    ///Vehicle pool for this faction. The Mission Creator palette lists these; placing a pool vehicle on the map is not wired up.
    pub vehicles: ::std::vec::Vec<Vehicle>,
}
///Display name, e.g. 'US Army 1980s'.
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct TbdFactionLibraryEntryName(::std::string::String);
impl ::std::ops::Deref for TbdFactionLibraryEntryName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<TbdFactionLibraryEntryName> for ::std::string::String {
    fn from(value: TbdFactionLibraryEntryName) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for TbdFactionLibraryEntryName {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 80usize {
            return Err("longer than 80 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for TbdFactionLibraryEntryName {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TbdFactionLibraryEntryName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TbdFactionLibraryEntryName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for TbdFactionLibraryEntryName {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///Export-side key (mirrors the mission doc Faction.key vocabulary).
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum TbdFactionLibraryEntrySide {
    #[serde(rename = "BLUFOR")]
    Blufor,
    #[serde(rename = "OPFOR")]
    Opfor,
    #[serde(rename = "INDFOR")]
    Indfor,
    #[serde(rename = "CIV")]
    Civ,
}
impl ::std::fmt::Display for TbdFactionLibraryEntrySide {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Blufor => f.write_str("BLUFOR"),
            Self::Opfor => f.write_str("OPFOR"),
            Self::Indfor => f.write_str("INDFOR"),
            Self::Civ => f.write_str("CIV"),
        }
    }
}
impl ::std::str::FromStr for TbdFactionLibraryEntrySide {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "BLUFOR" => Ok(Self::Blufor),
            "OPFOR" => Ok(Self::Opfor),
            "INDFOR" => Ok(Self::Indfor),
            "CIV" => Ok(Self::Civ),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for TbdFactionLibraryEntrySide {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TbdFactionLibraryEntrySide {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TbdFactionLibraryEntrySide {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
