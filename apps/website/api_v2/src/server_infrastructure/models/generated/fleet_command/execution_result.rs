// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/fleet-executor/commands/:commandId/result body. succeeded requires a prior executing report; a failure names failure_reason (1 to 512 bytes). list_players reports outcome {players:[{arma_id, name, ...}]}.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ExecutionResult {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub failure_reason: ::std::option::Option<ExecutionResultFailureReason>,
    pub fencing_token: ::std::num::NonZeroU64,
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub outcome: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    pub succeeded: bool,
}
///`ExecutionResultFailureReason`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ExecutionResultFailureReason(::std::string::String);
impl ::std::ops::Deref for ExecutionResultFailureReason {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ExecutionResultFailureReason> for ::std::string::String {
    fn from(value: ExecutionResultFailureReason) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ExecutionResultFailureReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 512usize {
            return Err("longer than 512 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ExecutionResultFailureReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ExecutionResultFailureReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ExecutionResultFailureReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ExecutionResultFailureReason {
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
