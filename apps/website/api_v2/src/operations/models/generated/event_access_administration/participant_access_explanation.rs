// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-access-administration.schema.json — regenerate with: cargo xtask ci schema-codegen

///One element of GET /api/v1/events/:id/access/participants (an array): why each participant is or is not eligible.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ParticipantAccessExplanation {
    pub allocation: ::std::option::Option<ParticipantAccessExplanationAllocation>,
    pub available: bool,
    pub discord_id: ::std::string::String,
    pub guilds: ::std::vec::Vec<ParticipantAccessExplanationGuildsItem>,
    pub registrations: ::std::vec::Vec<ParticipantAccessExplanationRegistrationsItem>,
    pub roster_groups: ::std::vec::Vec<ParticipantAccessExplanationRosterGroupsItem>,
    pub tbd_member: bool,
    pub username: ::std::string::String,
}
///`ParticipantAccessExplanationAllocation`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ParticipantAccessExplanationAllocation {
    pub acquired_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub quota_kind: ParticipantAccessExplanationAllocationQuotaKind,
}
///`ParticipantAccessExplanationAllocationQuotaKind`
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
pub enum ParticipantAccessExplanationAllocationQuotaKind {
    #[serde(rename = "member")]
    Member,
    #[serde(rename = "guest")]
    Guest,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "legacy_unclassified")]
    LegacyUnclassified,
}
impl ::std::fmt::Display for ParticipantAccessExplanationAllocationQuotaKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Member => f.write_str("member"),
            Self::Guest => f.write_str("guest"),
            Self::Open => f.write_str("open"),
            Self::LegacyUnclassified => f.write_str("legacy_unclassified"),
        }
    }
}
impl ::std::str::FromStr for ParticipantAccessExplanationAllocationQuotaKind {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            "open" => Ok(Self::Open),
            "legacy_unclassified" => Ok(Self::LegacyUnclassified),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ParticipantAccessExplanationAllocationQuotaKind {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for ParticipantAccessExplanationAllocationQuotaKind
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for ParticipantAccessExplanationAllocationQuotaKind
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`ParticipantAccessExplanationGuildsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ParticipantAccessExplanationGuildsItem {
    pub current: bool,
    pub guild_id: ::std::string::String,
    pub membership_status: ParticipantAccessExplanationGuildsItemMembershipStatus,
    pub override_until: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub verified_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
}
///`ParticipantAccessExplanationGuildsItemMembershipStatus`
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
pub enum ParticipantAccessExplanationGuildsItemMembershipStatus {
    #[serde(rename = "member")]
    Member,
    #[serde(rename = "nonmember")]
    Nonmember,
    #[serde(rename = "unknown")]
    Unknown,
}
impl ::std::fmt::Display for ParticipantAccessExplanationGuildsItemMembershipStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Member => f.write_str("member"),
            Self::Nonmember => f.write_str("nonmember"),
            Self::Unknown => f.write_str("unknown"),
        }
    }
}
impl ::std::str::FromStr for ParticipantAccessExplanationGuildsItemMembershipStatus {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "member" => Ok(Self::Member),
            "nonmember" => Ok(Self::Nonmember),
            "unknown" => Ok(Self::Unknown),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ParticipantAccessExplanationGuildsItemMembershipStatus {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for ParticipantAccessExplanationGuildsItemMembershipStatus
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for ParticipantAccessExplanationGuildsItemMembershipStatus
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`ParticipantAccessExplanationRegistrationsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ParticipantAccessExplanationRegistrationsItem {
    pub admitting_grants: ::std::vec::Vec<u64>,
    pub current_authority_admits: bool,
    pub event_mission_id: ::uuid::Uuid,
    pub last_verified_admits: bool,
    pub policy_source: ParticipantAccessExplanationRegistrationsItemPolicySource,
    pub queue_entered_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub registration_id: ::uuid::Uuid,
    pub reservation_state: ParticipantAccessExplanationRegistrationsItemReservationState,
    pub slot_id: ::std::option::Option<::uuid::Uuid>,
}
///`ParticipantAccessExplanationRegistrationsItemPolicySource`
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
pub enum ParticipantAccessExplanationRegistrationsItemPolicySource {
    #[serde(rename = "event")]
    Event,
    #[serde(rename = "squad")]
    Squad,
    #[serde(rename = "slot")]
    Slot,
}
impl ::std::fmt::Display for ParticipantAccessExplanationRegistrationsItemPolicySource {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Event => f.write_str("event"),
            Self::Squad => f.write_str("squad"),
            Self::Slot => f.write_str("slot"),
        }
    }
}
impl ::std::str::FromStr for ParticipantAccessExplanationRegistrationsItemPolicySource {
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
impl ::std::convert::TryFrom<&str> for ParticipantAccessExplanationRegistrationsItemPolicySource {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for ParticipantAccessExplanationRegistrationsItemPolicySource
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for ParticipantAccessExplanationRegistrationsItemPolicySource
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`ParticipantAccessExplanationRegistrationsItemReservationState`
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
pub enum ParticipantAccessExplanationRegistrationsItemReservationState {
    #[serde(rename = "registered")]
    Registered,
    #[serde(rename = "waitlisted")]
    Waitlisted,
    #[serde(rename = "withdrawn")]
    Withdrawn,
    #[serde(rename = "legacy_unknown")]
    LegacyUnknown,
}
impl ::std::fmt::Display for ParticipantAccessExplanationRegistrationsItemReservationState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Registered => f.write_str("registered"),
            Self::Waitlisted => f.write_str("waitlisted"),
            Self::Withdrawn => f.write_str("withdrawn"),
            Self::LegacyUnknown => f.write_str("legacy_unknown"),
        }
    }
}
impl ::std::str::FromStr for ParticipantAccessExplanationRegistrationsItemReservationState {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "registered" => Ok(Self::Registered),
            "waitlisted" => Ok(Self::Waitlisted),
            "withdrawn" => Ok(Self::Withdrawn),
            "legacy_unknown" => Ok(Self::LegacyUnknown),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str>
    for ParticipantAccessExplanationRegistrationsItemReservationState
{
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String>
    for ParticipantAccessExplanationRegistrationsItemReservationState
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String>
    for ParticipantAccessExplanationRegistrationsItemReservationState
{
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`ParticipantAccessExplanationRosterGroupsItem`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ParticipantAccessExplanationRosterGroupsItem {
    pub added_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub added_by: ::std::option::Option<::std::string::String>,
    pub group_id: ::uuid::Uuid,
    pub system_origin: ::std::option::Option<::std::string::String>,
}
