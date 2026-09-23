// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/faction-library.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::Slot;

///SlotLoadout v2 (mirrors loadout-export.schema.json v2 doc shapes).
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
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LoadoutV2CargoItem {
    pub container: LoadoutV2CargoItemContainer,
    pub item: LoadoutV2CargoItemItem,
    pub qty: ::std::num::NonZeroU64,
}
///`LoadoutV2CargoItemContainer`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2CargoItemContainer {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2CargoItemContainer {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2CargoItemContainer {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2CargoItemItem`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2CargoItemItem {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2CargoItemItem {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2CargoItemItem {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2EquipmentKey`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2EquipmentKey {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2EquipmentKey {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2WeaponsItem`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2WeaponsItemSlotType {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2WeaponsItemSlotType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2WeaponsItemSlotType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2WeaponsItemWeapon`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for LoadoutV2WeaponsItemWeapon {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2WeaponsItemWeapon {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2WeaponsItemWeapon {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
///`LoadoutV2WearKey`
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
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for LoadoutV2WearKey {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for LoadoutV2WearKey {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
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
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
