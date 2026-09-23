// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-session.schema.json — regenerate with: cargo xtask ci schema-codegen

///409 details of a refused heartbeat or deployment: code STALE_GENERATION (with generation), STALE_SEQUENCE (with last_sequence) or RUNTIME_SESSION_ENDED (with end_reason).
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFenceRefusal {
    pub code: RuntimeFenceRefusalCode,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub end_reason: ::std::option::Option<RuntimeFenceRefusalEndReason>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub generation: ::std::option::Option<i64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub last_sequence: ::std::option::Option<i64>,
}
///`RuntimeFenceRefusalCode`
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
pub enum RuntimeFenceRefusalCode {
    #[serde(rename = "STALE_GENERATION")]
    StaleGeneration,
    #[serde(rename = "STALE_SEQUENCE")]
    StaleSequence,
    #[serde(rename = "RUNTIME_SESSION_ENDED")]
    RuntimeSessionEnded,
}
impl ::std::fmt::Display for RuntimeFenceRefusalCode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::StaleGeneration => f.write_str("STALE_GENERATION"),
            Self::StaleSequence => f.write_str("STALE_SEQUENCE"),
            Self::RuntimeSessionEnded => f.write_str("RUNTIME_SESSION_ENDED"),
        }
    }
}
impl ::std::str::FromStr for RuntimeFenceRefusalCode {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "STALE_GENERATION" => Ok(Self::StaleGeneration),
            "STALE_SEQUENCE" => Ok(Self::StaleSequence),
            "RUNTIME_SESSION_ENDED" => Ok(Self::RuntimeSessionEnded),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for RuntimeFenceRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RuntimeFenceRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RuntimeFenceRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`RuntimeFenceRefusalEndReason`
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
pub enum RuntimeFenceRefusalEndReason {
    #[serde(rename = "superseded")]
    Superseded,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "ended_by_runtime")]
    EndedByRuntime,
    #[serde(rename = "credential_revoked")]
    CredentialRevoked,
}
impl ::std::fmt::Display for RuntimeFenceRefusalEndReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Superseded => f.write_str("superseded"),
            Self::Expired => f.write_str("expired"),
            Self::EndedByRuntime => f.write_str("ended_by_runtime"),
            Self::CredentialRevoked => f.write_str("credential_revoked"),
        }
    }
}
impl ::std::str::FromStr for RuntimeFenceRefusalEndReason {
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
impl ::std::convert::TryFrom<&str> for RuntimeFenceRefusalEndReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RuntimeFenceRefusalEndReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RuntimeFenceRefusalEndReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
