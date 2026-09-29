// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

///The outcome a succeeded console_command reports: the server's reply to the line, cut by the host agent on a character boundary at 4096 bytes, and whether it was cut. The API answers 400 for any other shape and for a response over 4096 bytes. A failed console_command may omit it; a line the server never answered fails with failure_reason "no RCON response; the command may or may not have run".
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ConsoleCommandOutcome {
    pub response: ConsoleCommandOutcomeResponse,
    pub response_truncated: bool,
}
///`ConsoleCommandOutcomeResponse`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ConsoleCommandOutcomeResponse(::std::string::String);
impl ::std::ops::Deref for ConsoleCommandOutcomeResponse {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ConsoleCommandOutcomeResponse> for ::std::string::String {
    fn from(value: ConsoleCommandOutcomeResponse) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ConsoleCommandOutcomeResponse {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 4096usize {
            return Err("longer than 4096 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ConsoleCommandOutcomeResponse {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ConsoleCommandOutcomeResponse {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ConsoleCommandOutcomeResponse {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ConsoleCommandOutcomeResponse {
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
