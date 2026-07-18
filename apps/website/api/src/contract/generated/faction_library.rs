// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: packages/tbd-schema/schema/faction-library.schema.json — regenerate with: make schema-codegen

/// Error types.
pub mod error {
    /// Error from a `TryFrom` or `FromStr` implementation.
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
///SlotLoadout v2 (mirrors loadout-export.schema.json v2 doc shapes).
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "SlotLoadout v2 (mirrors loadout-export.schema.json v2 doc shapes).",
///  "type": "object",
///  "required": [
///    "version",
///    "weapons",
///    "wear"
///  ],
///  "properties": {
///    "cargo": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "container",
///          "item",
///          "qty"
///        ],
///        "properties": {
///          "container": {
///            "type": "string",
///            "minLength": 1
///          },
///          "item": {
///            "type": "string",
///            "minLength": 1
///          },
///          "qty": {
///            "type": "integer",
///            "minimum": 1.0
///          }
///        },
///        "additionalProperties": false
///      }
///    },
///    "equipment": {
///      "type": "object",
///      "patternProperties": {
///        "^[a-zA-Z][a-zA-Z0-9_]{0,63}$": {
///          "$ref": "#/$defs/slot"
///        }
///      },
///      "additionalProperties": false
///    },
///    "summary": {
///      "type": "string"
///    },
///    "version": {
///      "description": "SlotLoadout v2 marker (const 2 — expressed as bounds for the quicktype Rust emitter).",
///      "type": "integer",
///      "maximum": 2.0,
///      "minimum": 2.0
///    },
///    "weapons": {
///      "type": "array",
///      "items": {
///        "type": "object",
///        "required": [
///          "slotIndex",
///          "slotType",
///          "weapon"
///        ],
///        "properties": {
///          "attachments": {
///            "type": "array",
///            "items": {
///              "type": "string"
///            }
///          },
///          "magazine": {
///            "$ref": "#/$defs/slot"
///          },
///          "optic": {
///            "$ref": "#/$defs/slot"
///          },
///          "slotIndex": {
///            "type": "integer",
///            "minimum": 0.0
///          },
///          "slotType": {
///            "type": "string",
///            "minLength": 1
///          },
///          "weapon": {
///            "type": "string",
///            "minLength": 1
///          }
///        },
///        "additionalProperties": false
///      }
///    },
///    "wear": {
///      "type": "object",
///      "patternProperties": {
///        "^[a-zA-Z][a-zA-Z0-9_]{0,63}$": {
///          "$ref": "#/$defs/slot"
///        }
///      },
///      "additionalProperties": false
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV2 {
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub cargo: ::std::vec::Vec<LoadoutV2CargoItem>,
    #[serde(
        default,
        skip_serializing_if = ":: std :: collections :: HashMap::is_empty"
    )]
    pub equipment: ::std::collections::HashMap<LoadoutV2EquipmentKey, Slot>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub summary: ::std::option::Option<::std::string::String>,
    ///SlotLoadout v2 marker (const 2 — expressed as bounds for the quicktype Rust emitter).
    pub version: i64,
    pub weapons: ::std::vec::Vec<LoadoutV2WeaponsItem>,
    pub wear: ::std::collections::HashMap<LoadoutV2WearKey, Slot>,
}
///`LoadoutV2CargoItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "container",
///    "item",
///    "qty"
///  ],
///  "properties": {
///    "container": {
///      "type": "string",
///      "minLength": 1
///    },
///    "item": {
///      "type": "string",
///      "minLength": 1
///    },
///    "qty": {
///      "type": "integer",
///      "minimum": 1.0
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV2CargoItem {
    pub container: LoadoutV2CargoItemContainer,
    pub item: LoadoutV2CargoItemItem,
    pub qty: ::std::num::NonZeroU64,
}
///`LoadoutV2CargoItemContainer`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct LoadoutV2CargoItemContainer(::std::string::String);
impl ::std::ops::Deref for LoadoutV2CargoItemContainer {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<LoadoutV2CargoItemContainer> for ::std::string::String {
    fn from(value: LoadoutV2CargoItemContainer) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for LoadoutV2CargoItemContainer {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2CargoItemContainer {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2CargoItemContainer {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2CargoItemContainer {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for LoadoutV2CargoItemContainer {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2CargoItemItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct LoadoutV2CargoItemItem(::std::string::String);
impl ::std::ops::Deref for LoadoutV2CargoItemItem {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<LoadoutV2CargoItemItem> for ::std::string::String {
    fn from(value: LoadoutV2CargoItemItem) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for LoadoutV2CargoItemItem {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2CargoItemItem {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2CargoItemItem {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2CargoItemItem {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for LoadoutV2CargoItemItem {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2EquipmentKey`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "pattern": "^[a-zA-Z][a-zA-Z0-9_]{0,63}$"
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct LoadoutV2EquipmentKey(::std::string::String);
impl ::std::ops::Deref for LoadoutV2EquipmentKey {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<LoadoutV2EquipmentKey> for ::std::string::String {
    fn from(value: LoadoutV2EquipmentKey) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for LoadoutV2EquipmentKey {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| {
                ::regress::Regex::new("^[a-zA-Z][a-zA-Z0-9_]{0,63}$").unwrap()
            });
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^[a-zA-Z][a-zA-Z0-9_]{0,63}$\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2EquipmentKey {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2EquipmentKey {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2EquipmentKey {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for LoadoutV2EquipmentKey {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2WeaponsItem`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "slotIndex",
///    "slotType",
///    "weapon"
///  ],
///  "properties": {
///    "attachments": {
///      "type": "array",
///      "items": {
///        "type": "string"
///      }
///    },
///    "magazine": {
///      "$ref": "#/$defs/slot"
///    },
///    "optic": {
///      "$ref": "#/$defs/slot"
///    },
///    "slotIndex": {
///      "type": "integer",
///      "minimum": 0.0
///    },
///    "slotType": {
///      "type": "string",
///      "minLength": 1
///    },
///    "weapon": {
///      "type": "string",
///      "minLength": 1
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV2WeaponsItem {
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub attachments: ::std::vec::Vec<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub magazine: ::std::option::Option<Slot>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub optic: ::std::option::Option<Slot>,
    #[serde(rename = "slotIndex")]
    pub slot_index: u64,
    #[serde(rename = "slotType")]
    pub slot_type: LoadoutV2WeaponsItemSlotType,
    pub weapon: LoadoutV2WeaponsItemWeapon,
}
///`LoadoutV2WeaponsItemSlotType`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct LoadoutV2WeaponsItemSlotType(::std::string::String);
impl ::std::ops::Deref for LoadoutV2WeaponsItemSlotType {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<LoadoutV2WeaponsItemSlotType> for ::std::string::String {
    fn from(value: LoadoutV2WeaponsItemSlotType) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for LoadoutV2WeaponsItemSlotType {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2WeaponsItemSlotType {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2WeaponsItemSlotType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2WeaponsItemSlotType {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for LoadoutV2WeaponsItemSlotType {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2WeaponsItemWeapon`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct LoadoutV2WeaponsItemWeapon(::std::string::String);
impl ::std::ops::Deref for LoadoutV2WeaponsItemWeapon {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<LoadoutV2WeaponsItemWeapon> for ::std::string::String {
    fn from(value: LoadoutV2WeaponsItemWeapon) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for LoadoutV2WeaponsItemWeapon {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2WeaponsItemWeapon {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2WeaponsItemWeapon {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2WeaponsItemWeapon {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for LoadoutV2WeaponsItemWeapon {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2WearKey`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "pattern": "^[a-zA-Z][a-zA-Z0-9_]{0,63}$"
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct LoadoutV2WearKey(::std::string::String);
impl ::std::ops::Deref for LoadoutV2WearKey {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<LoadoutV2WearKey> for ::std::string::String {
    fn from(value: LoadoutV2WearKey) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for LoadoutV2WearKey {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| {
                ::regress::Regex::new("^[a-zA-Z][a-zA-Z0-9_]{0,63}$").unwrap()
            });
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^[a-zA-Z][a-zA-Z0-9_]{0,63}$\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2WearKey {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2WearKey {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2WearKey {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for LoadoutV2WearKey {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///Full Enfusion ResourceName ({GUID}Prefabs/.../File.et).
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Full Enfusion ResourceName ({GUID}Prefabs/.../File.et).",
///  "type": "string",
///  "pattern": "^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$"
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ResourceName(::std::string::String);
impl ::std::ops::Deref for ResourceName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ResourceName> for ::std::string::String {
    fn from(value: ResourceName) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ResourceName {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| {
                ::regress::Regex::new("^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$").unwrap()
            });
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^\\{[0-9A-F]{16}\\}[A-Za-z0-9/_.\\- ()']+$\"".into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ResourceName {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ResourceName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ResourceName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ResourceName {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`Role`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "character",
///    "role"
///  ],
///  "properties": {
///    "character": {
///      "description": "Registry character (kind === character) this role wraps — vanilla bodies are fine here; the palette hides them, roles don't.",
///      "$ref": "#/$defs/resourceName"
///    },
///    "loadout": {
///      "$ref": "#/$defs/loadoutV2"
///    },
///    "role": {
///      "type": "string",
///      "maxLength": 60,
///      "minLength": 1
///    },
///    "tag": {
///      "type": "string",
///      "maxLength": 12
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Role {
    ///Registry character (kind === character) this role wraps — vanilla bodies are fine here; the palette hides them, roles don't.
    pub character: ResourceName,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub loadout: ::std::option::Option<LoadoutV2>,
    pub role: RoleRole,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub tag: ::std::option::Option<RoleTag>,
}
///`RoleRole`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "maxLength": 60,
///  "minLength": 1
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct RoleRole(::std::string::String);
impl ::std::ops::Deref for RoleRole {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<RoleRole> for ::std::string::String {
    fn from(value: RoleRole) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for RoleRole {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() > 60usize {
            return Err("longer than 60 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for RoleRole {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RoleRole {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RoleRole {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for RoleRole {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`RoleTag`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "maxLength": 12
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct RoleTag(::std::string::String);
impl ::std::ops::Deref for RoleTag {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<RoleTag> for ::std::string::String {
    fn from(value: RoleTag) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for RoleTag {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() > 12usize {
            return Err("longer than 12 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for RoleTag {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RoleTag {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RoleTag {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for RoleTag {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`Slot`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": [
///    "string",
///    "null"
///  ]
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Slot(pub ::std::option::Option<::std::string::String>);
impl ::std::ops::Deref for Slot {
    type Target = ::std::option::Option<::std::string::String>;
    fn deref(&self) -> &::std::option::Option<::std::string::String> {
        &self.0
    }
}
impl ::std::convert::From<Slot> for ::std::option::Option<::std::string::String> {
    fn from(value: Slot) -> Self {
        value.0
    }
}
impl ::std::convert::From<::std::option::Option<::std::string::String>> for Slot {
    fn from(value: ::std::option::Option<::std::string::String>) -> Self {
        Self(value)
    }
}
///One operator-authored reusable faction (T-153): a side + display name plus its ORBAT role templates (each wrapping a registry character with an optional SlotLoadout v2) and its vehicle pool. Stored as the jsonb doc of a user_factions row; the Mission Creator palette renders side → faction → roles/vehicles from these instead of the raw vanilla registry dump. Role loadouts reuse the loadout-export v2 shapes (wear open map keyed by engine LoadoutSlotInfo name; slot-indexed weapons).
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "$id": "https://schema.tbdevent.eu/faction-library/v1.json",
///  "title": "TBD Faction Library Entry",
///  "description": "One operator-authored reusable faction (T-153): a side + display name plus its ORBAT role templates (each wrapping a registry character with an optional SlotLoadout v2) and its vehicle pool. Stored as the jsonb doc of a user_factions row; the Mission Creator palette renders side → faction → roles/vehicles from these instead of the raw vanilla registry dump. Role loadouts reuse the loadout-export v2 shapes (wear open map keyed by engine LoadoutSlotInfo name; slot-indexed weapons).",
///  "type": "object",
///  "required": [
///    "name",
///    "roles",
///    "side",
///    "vehicles"
///  ],
///  "properties": {
///    "emblem": {
///      "description": "Optional emblem asset path/URL (UI later).",
///      "type": "string"
///    },
///    "name": {
///      "description": "Display name, e.g. 'US Army 1980s'.",
///      "type": "string",
///      "maxLength": 80,
///      "minLength": 1
///    },
///    "roles": {
///      "description": "ORBAT role templates in authored order — the palette's draggable leaves.",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/role"
///      }
///    },
///    "side": {
///      "description": "Export-side key (mirrors the mission doc Faction.key vocabulary).",
///      "type": "string",
///      "enum": [
///        "BLUFOR",
///        "OPFOR",
///        "INDFOR",
///        "CIV"
///      ]
///    },
///    "vehicles": {
///      "description": "Vehicle pool (listed in the palette; map placement lands with T-070).",
///      "type": "array",
///      "items": {
///        "$ref": "#/$defs/vehicle"
///      }
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
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
    ///Vehicle pool (listed in the palette; map placement lands with T-070).
    pub vehicles: ::std::vec::Vec<Vehicle>,
}
///Display name, e.g. 'US Army 1980s'.
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Display name, e.g. 'US Army 1980s'.",
///  "type": "string",
///  "maxLength": 80,
///  "minLength": 1
///}
/// ```
/// </details>
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
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
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
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TbdFactionLibraryEntryName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TbdFactionLibraryEntryName {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
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
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///Export-side key (mirrors the mission doc Faction.key vocabulary).
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "description": "Export-side key (mirrors the mission doc Faction.key vocabulary).",
///  "type": "string",
///  "enum": [
///    "BLUFOR",
///    "OPFOR",
///    "INDFOR",
///    "CIV"
///  ]
///}
/// ```
/// </details>
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
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
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
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TbdFactionLibraryEntrySide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TbdFactionLibraryEntrySide {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
///`Vehicle`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "object",
///  "required": [
///    "vehicle"
///  ],
///  "properties": {
///    "label": {
///      "type": "string",
///      "maxLength": 60
///    },
///    "vehicle": {
///      "$ref": "#/$defs/resourceName"
///    }
///  },
///  "additionalProperties": false
///}
/// ```
/// </details>
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct Vehicle {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub label: ::std::option::Option<VehicleLabel>,
    pub vehicle: ResourceName,
}
///`VehicleLabel`
///
/// <details><summary>JSON schema</summary>
///
/// ```json
///{
///  "type": "string",
///  "maxLength": 60
///}
/// ```
/// </details>
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct VehicleLabel(::std::string::String);
impl ::std::ops::Deref for VehicleLabel {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<VehicleLabel> for ::std::string::String {
    fn from(value: VehicleLabel) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for VehicleLabel {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        if value.chars().count() > 60usize {
            return Err("longer than 60 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for VehicleLabel {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for VehicleLabel {
    type Error = self::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for VehicleLabel {
    type Error = self::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for VehicleLabel {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: self::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
