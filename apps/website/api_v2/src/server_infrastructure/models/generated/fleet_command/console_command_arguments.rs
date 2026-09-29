// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

///The arguments of console_command as the ledger stores them and the host agent receives them: one line for the server's RCON console. The API trims surrounding whitespace from the requested line and answers 400 for a line outside 1 to 256 bytes, one holding a control character or a line or paragraph separator, and one starting with @, which begins Reforger's custom RCON commands for the RCON session itself (@logout), a session the host agent owns. The host agent transmits the line at most once and never resends it.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ConsoleCommandArguments {
    pub line: ConsoleCommandArgumentsLine,
}
///`ConsoleCommandArgumentsLine`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ConsoleCommandArgumentsLine(::std::string::String);
impl ::std::ops::Deref for ConsoleCommandArgumentsLine {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ConsoleCommandArgumentsLine> for ::std::string::String {
    fn from(value: ConsoleCommandArgumentsLine) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ConsoleCommandArgumentsLine {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 256usize {
            return Err("longer than 256 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(
            || {
                ::regress::Regex::new(
                    "^[^@\\u0000-\\u001F\\u007F-\\u009F\\u2028\\u2029][^\\u0000-\\u001F\\u007F-\\u009F\\u2028\\u2029]*$",
                )
                .unwrap()
            },
        );
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^[^@\\u0000-\\u001F\\u007F-\\u009F\\u2028\\u2029][^\\u0000-\\u001F\\u007F-\\u009F\\u2028\\u2029]*$\""
                    .into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ConsoleCommandArgumentsLine {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ConsoleCommandArgumentsLine {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ConsoleCommandArgumentsLine {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ConsoleCommandArgumentsLine {
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
