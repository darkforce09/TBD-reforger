// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

///`MissionStatus`
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
pub enum MissionStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "pending_approval")]
    PendingApproval,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "archived")]
    Archived,
}
impl ::std::fmt::Display for MissionStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Draft => f.write_str("draft"),
            Self::PendingApproval => f.write_str("pending_approval"),
            Self::Live => f.write_str("live"),
            Self::Rejected => f.write_str("rejected"),
            Self::Archived => f.write_str("archived"),
        }
    }
}
impl ::std::str::FromStr for MissionStatus {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "draft" => Ok(Self::Draft),
            "pending_approval" => Ok(Self::PendingApproval),
            "live" => Ok(Self::Live),
            "rejected" => Ok(Self::Rejected),
            "archived" => Ok(Self::Archived),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for MissionStatus {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MissionStatus {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MissionStatus {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
