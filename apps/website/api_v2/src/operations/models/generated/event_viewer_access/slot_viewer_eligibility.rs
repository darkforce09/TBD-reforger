// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-viewer-access.schema.json — regenerate with: cargo xtask ci schema-codegen

///`SlotViewerEligibility`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct SlotViewerEligibility {
    pub policy_source: SlotViewerEligibilityPolicySource,
    pub viewer_access: SlotViewerEligibilityViewerAccess,
}
///`SlotViewerEligibilityPolicySource`
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
pub enum SlotViewerEligibilityPolicySource {
    #[serde(rename = "event")]
    Event,
    #[serde(rename = "squad")]
    Squad,
    #[serde(rename = "slot")]
    Slot,
}
impl ::std::fmt::Display for SlotViewerEligibilityPolicySource {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Event => f.write_str("event"),
            Self::Squad => f.write_str("squad"),
            Self::Slot => f.write_str("slot"),
        }
    }
}
impl ::std::str::FromStr for SlotViewerEligibilityPolicySource {
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
impl ::std::convert::TryFrom<&str> for SlotViewerEligibilityPolicySource {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for SlotViewerEligibilityPolicySource {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for SlotViewerEligibilityPolicySource {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`SlotViewerEligibilityViewerAccess`
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
pub enum SlotViewerEligibilityViewerAccess {
    #[serde(rename = "eligible")]
    Eligible,
    #[serde(rename = "restricted")]
    Restricted,
}
impl ::std::fmt::Display for SlotViewerEligibilityViewerAccess {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Eligible => f.write_str("eligible"),
            Self::Restricted => f.write_str("restricted"),
        }
    }
}
impl ::std::str::FromStr for SlotViewerEligibilityViewerAccess {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "eligible" => Ok(Self::Eligible),
            "restricted" => Ok(Self::Restricted),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for SlotViewerEligibilityViewerAccess {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for SlotViewerEligibilityViewerAccess {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for SlotViewerEligibilityViewerAccess {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
