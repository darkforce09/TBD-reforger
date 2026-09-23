// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-session.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/game-runtime/sessions/:sessionId/end response. Ending an ended session reports how it ended.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSessionEnd {
    pub end_reason: RuntimeSessionEndEndReason,
    pub ended: bool,
    pub runtime_session_id: ::uuid::Uuid,
}
///`RuntimeSessionEndEndReason`
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
pub enum RuntimeSessionEndEndReason {
    #[serde(rename = "superseded")]
    Superseded,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "ended_by_runtime")]
    EndedByRuntime,
    #[serde(rename = "credential_revoked")]
    CredentialRevoked,
}
impl ::std::fmt::Display for RuntimeSessionEndEndReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Superseded => f.write_str("superseded"),
            Self::Expired => f.write_str("expired"),
            Self::EndedByRuntime => f.write_str("ended_by_runtime"),
            Self::CredentialRevoked => f.write_str("credential_revoked"),
        }
    }
}
impl ::std::str::FromStr for RuntimeSessionEndEndReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "superseded" => Ok(Self::Superseded),
            "expired" => Ok(Self::Expired),
            "ended_by_runtime" => Ok(Self::EndedByRuntime),
            "credential_revoked" => Ok(Self::CredentialRevoked),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for RuntimeSessionEndEndReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RuntimeSessionEndEndReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RuntimeSessionEndEndReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
