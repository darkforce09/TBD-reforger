// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///An open life: what entitled the player to the slot, and when the life started.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DeploymentAllowed {
    pub arma_id: DeploymentAllowedArmaId,
    pub authorized_by: DeploymentAllowedAuthorizedBy,
    pub decision: ::std::string::String,
    pub event_mission_id: ::uuid::Uuid,
    pub occupancy_id: ::uuid::Uuid,
    pub orbat_slot_id: ::uuid::Uuid,
    pub player_life_id: DeploymentAllowedPlayerLifeId,
    pub runtime_session_id: ::uuid::Uuid,
    pub started_at: ::chrono::DateTime<::chrono::offset::Utc>,
}
///`DeploymentAllowedArmaId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct DeploymentAllowedArmaId(::std::string::String);
impl ::std::ops::Deref for DeploymentAllowedArmaId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<DeploymentAllowedArmaId> for ::std::string::String {
    fn from(value: DeploymentAllowedArmaId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for DeploymentAllowedArmaId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 128usize {
            return Err("longer than 128 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for DeploymentAllowedArmaId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DeploymentAllowedArmaId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DeploymentAllowedArmaId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for DeploymentAllowedArmaId {
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
///`DeploymentAllowedAuthorizedBy`
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
pub enum DeploymentAllowedAuthorizedBy {
    #[serde(rename = "reservation")]
    Reservation,
    #[serde(rename = "open_slot_policy")]
    OpenSlotPolicy,
}
impl ::std::fmt::Display for DeploymentAllowedAuthorizedBy {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Reservation => f.write_str("reservation"),
            Self::OpenSlotPolicy => f.write_str("open_slot_policy"),
        }
    }
}
impl ::std::str::FromStr for DeploymentAllowedAuthorizedBy {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "reservation" => Ok(Self::Reservation),
            "open_slot_policy" => Ok(Self::OpenSlotPolicy),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for DeploymentAllowedAuthorizedBy {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DeploymentAllowedAuthorizedBy {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DeploymentAllowedAuthorizedBy {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`DeploymentAllowedPlayerLifeId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct DeploymentAllowedPlayerLifeId(::std::string::String);
impl ::std::ops::Deref for DeploymentAllowedPlayerLifeId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<DeploymentAllowedPlayerLifeId> for ::std::string::String {
    fn from(value: DeploymentAllowedPlayerLifeId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for DeploymentAllowedPlayerLifeId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 128usize {
            return Err("longer than 128 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for DeploymentAllowedPlayerLifeId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DeploymentAllowedPlayerLifeId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DeploymentAllowedPlayerLifeId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for DeploymentAllowedPlayerLifeId {
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
