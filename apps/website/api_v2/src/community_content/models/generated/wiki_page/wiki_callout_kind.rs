// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

///The kind of a callout. The GitHub alerts give note, tip, important, warning and caution; the legacy markers add info and critical.
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
pub enum WikiCalloutKind {
    #[serde(rename = "note")]
    Note,
    #[serde(rename = "tip")]
    Tip,
    #[serde(rename = "important")]
    Important,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "caution")]
    Caution,
    #[serde(rename = "critical")]
    Critical,
}
impl ::std::fmt::Display for WikiCalloutKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Note => f.write_str("note"),
            Self::Tip => f.write_str("tip"),
            Self::Important => f.write_str("important"),
            Self::Info => f.write_str("info"),
            Self::Warning => f.write_str("warning"),
            Self::Caution => f.write_str("caution"),
            Self::Critical => f.write_str("critical"),
        }
    }
}
impl ::std::str::FromStr for WikiCalloutKind {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "note" => Ok(Self::Note),
            "tip" => Ok(Self::Tip),
            "important" => Ok(Self::Important),
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "caution" => Ok(Self::Caution),
            "critical" => Ok(Self::Critical),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for WikiCalloutKind {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiCalloutKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiCalloutKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
