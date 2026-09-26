// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{
    CombatDeathPayload, CombatKillPayload, MedicalIncapacitatedPayload, MedicalRevivedPayload,
    VehicleDestroyedPayload, VehicleEnteredPayload, VehicleExitedPayload,
};

///One detailed event. event_id and sequence are unique within the match; sequence is the capture order and the order of every read.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum MatchEvent {
    ///CombatKillEvent
    #[serde(rename = "combat.kill")]
    CombatKill {
        event_id: MatchEventEventId,
        mission_time_ms: u64,
        occurred_at: ::chrono::DateTime<::chrono::offset::Utc>,
        payload: CombatKillPayload,
        sequence: ::std::num::NonZeroU64,
    },
    ///CombatDeathEvent
    #[serde(rename = "combat.death")]
    CombatDeath {
        event_id: MatchEventEventId,
        mission_time_ms: u64,
        occurred_at: ::chrono::DateTime<::chrono::offset::Utc>,
        payload: CombatDeathPayload,
        sequence: ::std::num::NonZeroU64,
    },
    ///MedicalIncapacitatedEvent
    #[serde(rename = "medical.incapacitated")]
    MedicalIncapacitated {
        event_id: MatchEventEventId,
        mission_time_ms: u64,
        occurred_at: ::chrono::DateTime<::chrono::offset::Utc>,
        payload: MedicalIncapacitatedPayload,
        sequence: ::std::num::NonZeroU64,
    },
    ///MedicalRevivedEvent
    #[serde(rename = "medical.revived")]
    MedicalRevived {
        event_id: MatchEventEventId,
        mission_time_ms: u64,
        occurred_at: ::chrono::DateTime<::chrono::offset::Utc>,
        payload: MedicalRevivedPayload,
        sequence: ::std::num::NonZeroU64,
    },
    ///VehicleDestroyedEvent
    #[serde(rename = "vehicle.destroyed")]
    VehicleDestroyed {
        event_id: MatchEventEventId,
        mission_time_ms: u64,
        occurred_at: ::chrono::DateTime<::chrono::offset::Utc>,
        payload: VehicleDestroyedPayload,
        sequence: ::std::num::NonZeroU64,
    },
    ///VehicleEnteredEvent
    #[serde(rename = "vehicle.entered")]
    VehicleEntered {
        event_id: MatchEventEventId,
        mission_time_ms: u64,
        occurred_at: ::chrono::DateTime<::chrono::offset::Utc>,
        payload: VehicleEnteredPayload,
        sequence: ::std::num::NonZeroU64,
    },
    ///VehicleExitedEvent
    #[serde(rename = "vehicle.exited")]
    VehicleExited {
        event_id: MatchEventEventId,
        mission_time_ms: u64,
        occurred_at: ::chrono::DateTime<::chrono::offset::Utc>,
        payload: VehicleExitedPayload,
        sequence: ::std::num::NonZeroU64,
    },
}
///`MatchEventEventId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MatchEventEventId(::std::string::String);
impl ::std::ops::Deref for MatchEventEventId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MatchEventEventId> for ::std::string::String {
    fn from(value: MatchEventEventId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MatchEventEventId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| {
                ::regress::Regex::new("^[A-Za-z0-9._:-]{1,64}$").unwrap()
            });
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^[A-Za-z0-9._:-]{1,64}$\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MatchEventEventId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MatchEventEventId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MatchEventEventId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MatchEventEventId {
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
