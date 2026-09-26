// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/ingest/matches body (mod_runtime machine credential). Registers a server-scoped source match before any report about it. The same body again is inert; another body for a registered source answers 409 REGISTRATION_CONFLICT. server_id is refused: the credential names the server.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchRegistration {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub event_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub mission_id: ::std::option::Option<::uuid::Uuid>,
    pub runtime_session_id: ::uuid::Uuid,
    pub source_match_id: MatchRegistrationSourceMatchId,
    pub started_at: ::chrono::DateTime<::chrono::offset::Utc>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub terrain: ::std::option::Option<MatchRegistrationTerrain>,
}
///`MatchRegistrationSourceMatchId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MatchRegistrationSourceMatchId(::std::string::String);
impl ::std::ops::Deref for MatchRegistrationSourceMatchId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MatchRegistrationSourceMatchId> for ::std::string::String {
    fn from(value: MatchRegistrationSourceMatchId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MatchRegistrationSourceMatchId {
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
impl ::std::convert::TryFrom<&str> for MatchRegistrationSourceMatchId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MatchRegistrationSourceMatchId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MatchRegistrationSourceMatchId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MatchRegistrationSourceMatchId {
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
///`MatchRegistrationTerrain`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MatchRegistrationTerrain(::std::string::String);
impl ::std::ops::Deref for MatchRegistrationTerrain {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MatchRegistrationTerrain> for ::std::string::String {
    fn from(value: MatchRegistrationTerrain) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MatchRegistrationTerrain {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MatchRegistrationTerrain {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MatchRegistrationTerrain {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MatchRegistrationTerrain {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MatchRegistrationTerrain {
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
