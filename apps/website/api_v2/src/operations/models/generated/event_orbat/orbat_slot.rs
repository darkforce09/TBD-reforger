// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-orbat.schema.json — regenerate with: cargo xtask ci schema-codegen

///`OrbatSlot`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct OrbatSlot {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub assigned_name: ::std::option::Option<::std::string::String>,
    pub assigned_to: ::std::option::Option<::std::string::String>,
    pub id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub loadout: ::std::option::Option<::std::string::String>,
    pub number: i64,
    pub policy_source: OrbatSlotPolicySource,
    pub role: ::std::string::String,
    pub slot_index: i64,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub tag: ::std::option::Option<::std::string::String>,
    pub viewer_access: OrbatSlotViewerAccess,
}
///`OrbatSlotPolicySource`
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
pub enum OrbatSlotPolicySource {
    #[serde(rename = "event")]
    Event,
    #[serde(rename = "squad")]
    Squad,
    #[serde(rename = "slot")]
    Slot,
}
impl ::std::fmt::Display for OrbatSlotPolicySource {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Event => f.write_str("event"),
            Self::Squad => f.write_str("squad"),
            Self::Slot => f.write_str("slot"),
        }
    }
}
impl ::std::str::FromStr for OrbatSlotPolicySource {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "event" => Ok(Self::Event),
            "squad" => Ok(Self::Squad),
            "slot" => Ok(Self::Slot),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for OrbatSlotPolicySource {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for OrbatSlotPolicySource {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for OrbatSlotPolicySource {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`OrbatSlotViewerAccess`
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
pub enum OrbatSlotViewerAccess {
    #[serde(rename = "eligible")]
    Eligible,
    #[serde(rename = "restricted")]
    Restricted,
}
impl ::std::fmt::Display for OrbatSlotViewerAccess {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Eligible => f.write_str("eligible"),
            Self::Restricted => f.write_str("restricted"),
        }
    }
}
impl ::std::str::FromStr for OrbatSlotViewerAccess {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "eligible" => Ok(Self::Eligible),
            "restricted" => Ok(Self::Restricted),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for OrbatSlotViewerAccess {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for OrbatSlotViewerAccess {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for OrbatSlotViewerAccess {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
