// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-hub.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{EventMissionDossier, EventViewerAccess, ReservationQuotaAvailability};

///GET /api/v1/events/:id: the event dossier projected for the viewer. A hidden event answers 404 exactly like a missing one. A partial viewer receives only admitted missions and no event briefing. remaining_event_places is null when the event is uncapped.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventHub {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub banner_image_url: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub briefing: ::std::option::Option<::std::string::String>,
    pub created_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub created_by: ::std::string::String,
    pub id: ::uuid::Uuid,
    pub max_slots: u64,
    pub missions: ::std::vec::Vec<EventMissionDossier>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub modpack_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub name_override: ::std::option::Option<::std::string::String>,
    pub registration_locked: bool,
    pub remaining_event_places: ::std::option::Option<u64>,
    pub reservation_quotas: [ReservationQuotaAvailability; 3usize],
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub server_id: ::std::option::Option<::uuid::Uuid>,
    pub start_time: ::chrono::DateTime<::chrono::offset::Utc>,
    pub status: EventHubStatus,
    pub updated_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub viewer_access: EventViewerAccess,
}
///`EventHubStatus`
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
pub enum EventHubStatus {
    #[serde(rename = "scheduled")]
    Scheduled,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "locked")]
    Locked,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "cancelled")]
    Cancelled,
}
impl ::std::fmt::Display for EventHubStatus {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Scheduled => f.write_str("scheduled"),
            Self::Open => f.write_str("open"),
            Self::Locked => f.write_str("locked"),
            Self::Live => f.write_str("live"),
            Self::Completed => f.write_str("completed"),
            Self::Cancelled => f.write_str("cancelled"),
        }
    }
}
impl ::std::str::FromStr for EventHubStatus {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "scheduled" => Ok(Self::Scheduled),
            "open" => Ok(Self::Open),
            "locked" => Ok(Self::Locked),
            "live" => Ok(Self::Live),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventHubStatus {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventHubStatus {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventHubStatus {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
