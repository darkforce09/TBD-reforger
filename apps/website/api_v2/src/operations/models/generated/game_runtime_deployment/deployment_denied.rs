// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///A refusal and its reason; occupancy_id names the open life that blocks a player already deployed.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DeploymentDenied {
    pub decision: ::std::string::String,
    pub message: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub occupancy_id: ::std::option::Option<::uuid::Uuid>,
    pub reason: DeploymentDeniedReason,
}
///`DeploymentDeniedReason`
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
pub enum DeploymentDeniedReason {
    #[serde(rename = "IDENTITY_NOT_LINKED")]
    IdentityNotLinked,
    #[serde(rename = "ACCOUNT_UNAVAILABLE")]
    AccountUnavailable,
    #[serde(rename = "SLOT_RESERVED")]
    SlotReserved,
    #[serde(rename = "RESERVED_ANOTHER_SLOT")]
    ReservedAnotherSlot,
    #[serde(rename = "ACCESS_POLICY")]
    AccessPolicy,
    #[serde(rename = "MEMBERSHIP_VERIFICATION_REQUIRED")]
    MembershipVerificationRequired,
    #[serde(rename = "LIVE_SLOT_OCCUPIED")]
    LiveSlotOccupied,
    #[serde(rename = "PLAYER_ALREADY_DEPLOYED")]
    PlayerAlreadyDeployed,
    #[serde(rename = "SLOT_NOT_IN_LOADED_MISSION")]
    SlotNotInLoadedMission,
}
impl ::std::fmt::Display for DeploymentDeniedReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::IdentityNotLinked => f.write_str("IDENTITY_NOT_LINKED"),
            Self::AccountUnavailable => f.write_str("ACCOUNT_UNAVAILABLE"),
            Self::SlotReserved => f.write_str("SLOT_RESERVED"),
            Self::ReservedAnotherSlot => f.write_str("RESERVED_ANOTHER_SLOT"),
            Self::AccessPolicy => f.write_str("ACCESS_POLICY"),
            Self::MembershipVerificationRequired => f.write_str("MEMBERSHIP_VERIFICATION_REQUIRED"),
            Self::LiveSlotOccupied => f.write_str("LIVE_SLOT_OCCUPIED"),
            Self::PlayerAlreadyDeployed => f.write_str("PLAYER_ALREADY_DEPLOYED"),
            Self::SlotNotInLoadedMission => f.write_str("SLOT_NOT_IN_LOADED_MISSION"),
        }
    }
}
impl ::std::str::FromStr for DeploymentDeniedReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "IDENTITY_NOT_LINKED" => Ok(Self::IdentityNotLinked),
            "ACCOUNT_UNAVAILABLE" => Ok(Self::AccountUnavailable),
            "SLOT_RESERVED" => Ok(Self::SlotReserved),
            "RESERVED_ANOTHER_SLOT" => Ok(Self::ReservedAnotherSlot),
            "ACCESS_POLICY" => Ok(Self::AccessPolicy),
            "MEMBERSHIP_VERIFICATION_REQUIRED" => Ok(Self::MembershipVerificationRequired),
            "LIVE_SLOT_OCCUPIED" => Ok(Self::LiveSlotOccupied),
            "PLAYER_ALREADY_DEPLOYED" => Ok(Self::PlayerAlreadyDeployed),
            "SLOT_NOT_IN_LOADED_MISSION" => Ok(Self::SlotNotInLoadedMission),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for DeploymentDeniedReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DeploymentDeniedReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DeploymentDeniedReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
