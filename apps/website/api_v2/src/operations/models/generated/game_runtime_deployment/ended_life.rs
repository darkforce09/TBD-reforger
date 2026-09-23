// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/game-runtime/sessions/:sessionId/deployments/:occupancyId/end response. Ending names exactly one life; repeating it reports how that life ended.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EndedLife {
    pub end_reason: EndedLifeEndReason,
    pub ended: bool,
    pub occupancy_id: ::uuid::Uuid,
}
///`EndedLifeEndReason`
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
pub enum EndedLifeEndReason {
    #[serde(rename = "life_ended")]
    LifeEnded,
    #[serde(rename = "session_superseded")]
    SessionSuperseded,
    #[serde(rename = "session_expired")]
    SessionExpired,
    #[serde(rename = "session_ended")]
    SessionEnded,
    #[serde(rename = "credential_revoked")]
    CredentialRevoked,
}
impl ::std::fmt::Display for EndedLifeEndReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::LifeEnded => f.write_str("life_ended"),
            Self::SessionSuperseded => f.write_str("session_superseded"),
            Self::SessionExpired => f.write_str("session_expired"),
            Self::SessionEnded => f.write_str("session_ended"),
            Self::CredentialRevoked => f.write_str("credential_revoked"),
        }
    }
}
impl ::std::str::FromStr for EndedLifeEndReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "life_ended" => Ok(Self::LifeEnded),
            "session_superseded" => Ok(Self::SessionSuperseded),
            "session_expired" => Ok(Self::SessionExpired),
            "session_ended" => Ok(Self::SessionEnded),
            "credential_revoked" => Ok(Self::CredentialRevoked),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EndedLifeEndReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EndedLifeEndReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EndedLifeEndReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
