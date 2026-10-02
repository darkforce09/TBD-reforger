// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/vehicle-database.schema.json — regenerate with: cargo xtask ci schema-codegen

///PATCH /api/v1/vehicle-database/{id} body (administrator, answered 200): an absent key leaves its field unchanged, and null or an empty value clears an optional field; null or a blank value for name, faction or armor_type answers 400. The limits and trimming of VehicleWrite apply, unknown keys answer 400, and the write appends an audit line in its transaction. DELETE /api/v1/vehicle-database/{id} (administrator) soft-deletes the row and answers 200 with it.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct VehiclePatch {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub amphibious: ::std::option::Option<VehiclePatchAmphibious>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub armor_type: ::std::option::Option<VehiclePatchArmorType>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub faction: ::std::option::Option<VehiclePatchFaction>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub name: ::std::option::Option<VehiclePatchName>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub primary_threat: ::std::option::Option<VehiclePatchPrimaryThreat>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub profile_image_url: ::std::option::Option<VehiclePatchProfileImageUrl>,
}
impl ::std::default::Default for VehiclePatch {
    fn default() -> Self {
        Self {
            amphibious: Default::default(),
            armor_type: Default::default(),
            faction: Default::default(),
            name: Default::default(),
            primary_threat: Default::default(),
            profile_image_url: Default::default(),
        }
    }
}
///`VehiclePatchAmphibious`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct VehiclePatchAmphibious(::std::string::String);
impl ::std::ops::Deref for VehiclePatchAmphibious {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<VehiclePatchAmphibious> for ::std::string::String {
    fn from(value: VehiclePatchAmphibious) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for VehiclePatchAmphibious {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 60usize {
            return Err("longer than 60 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for VehiclePatchAmphibious {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for VehiclePatchAmphibious {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for VehiclePatchAmphibious {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for VehiclePatchAmphibious {
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
///`VehiclePatchArmorType`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct VehiclePatchArmorType(::std::string::String);
impl ::std::ops::Deref for VehiclePatchArmorType {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<VehiclePatchArmorType> for ::std::string::String {
    fn from(value: VehiclePatchArmorType) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for VehiclePatchArmorType {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 60usize {
            return Err("longer than 60 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("\\S").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"\\S\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for VehiclePatchArmorType {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for VehiclePatchArmorType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for VehiclePatchArmorType {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for VehiclePatchArmorType {
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
///`VehiclePatchFaction`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct VehiclePatchFaction(::std::string::String);
impl ::std::ops::Deref for VehiclePatchFaction {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<VehiclePatchFaction> for ::std::string::String {
    fn from(value: VehiclePatchFaction) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for VehiclePatchFaction {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 60usize {
            return Err("longer than 60 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("\\S").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"\\S\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for VehiclePatchFaction {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for VehiclePatchFaction {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for VehiclePatchFaction {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for VehiclePatchFaction {
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
///`VehiclePatchName`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct VehiclePatchName(::std::string::String);
impl ::std::ops::Deref for VehiclePatchName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<VehiclePatchName> for ::std::string::String {
    fn from(value: VehiclePatchName) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for VehiclePatchName {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 120usize {
            return Err("longer than 120 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("\\S").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"\\S\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for VehiclePatchName {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for VehiclePatchName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for VehiclePatchName {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for VehiclePatchName {
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
///`VehiclePatchPrimaryThreat`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct VehiclePatchPrimaryThreat(::std::string::String);
impl ::std::ops::Deref for VehiclePatchPrimaryThreat {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<VehiclePatchPrimaryThreat> for ::std::string::String {
    fn from(value: VehiclePatchPrimaryThreat) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for VehiclePatchPrimaryThreat {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 120usize {
            return Err("longer than 120 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for VehiclePatchPrimaryThreat {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for VehiclePatchPrimaryThreat {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for VehiclePatchPrimaryThreat {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for VehiclePatchPrimaryThreat {
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
///`VehiclePatchProfileImageUrl`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct VehiclePatchProfileImageUrl(::std::string::String);
impl ::std::ops::Deref for VehiclePatchProfileImageUrl {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<VehiclePatchProfileImageUrl> for ::std::string::String {
    fn from(value: VehiclePatchProfileImageUrl) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for VehiclePatchProfileImageUrl {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(
            || {
                ::regress::Regex::new(
                    "^(|https://[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|/[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*)$",
                )
                .unwrap()
            },
        );
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^(|https://[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*|/[^\\s\\\\/\\x00-\\x1F\\x7F-\\x9F][^\\s\\\\\\x00-\\x1F\\x7F-\\x9F]*)$\""
                    .into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for VehiclePatchProfileImageUrl {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for VehiclePatchProfileImageUrl {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for VehiclePatchProfileImageUrl {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for VehiclePatchProfileImageUrl {
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
