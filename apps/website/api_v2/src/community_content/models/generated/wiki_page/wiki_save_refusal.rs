// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::WikiMarkupFinding;

///The details of a refused wiki save, under the error envelope's details: wiki_revision_conflict (409) carries the page's current_revision, wiki_body_too_large (400) refuses a body over 262 144 bytes, and wiki_markup_refused (422) lists every finding.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WikiSaveRefusal {
    pub code: WikiSaveRefusalCode,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub current_revision: ::std::option::Option<::std::num::NonZeroU64>,
    #[serde(default, skip_serializing_if = "::std::vec::Vec::is_empty")]
    pub findings: ::std::vec::Vec<WikiMarkupFinding>,
}
///`WikiSaveRefusalCode`
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
pub enum WikiSaveRefusalCode {
    #[serde(rename = "wiki_revision_conflict")]
    WikiRevisionConflict,
    #[serde(rename = "wiki_body_too_large")]
    WikiBodyTooLarge,
    #[serde(rename = "wiki_markup_refused")]
    WikiMarkupRefused,
}
impl ::std::fmt::Display for WikiSaveRefusalCode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::WikiRevisionConflict => f.write_str("wiki_revision_conflict"),
            Self::WikiBodyTooLarge => f.write_str("wiki_body_too_large"),
            Self::WikiMarkupRefused => f.write_str("wiki_markup_refused"),
        }
    }
}
impl ::std::str::FromStr for WikiSaveRefusalCode {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "wiki_revision_conflict" => Ok(Self::WikiRevisionConflict),
            "wiki_body_too_large" => Ok(Self::WikiBodyTooLarge),
            "wiki_markup_refused" => Ok(Self::WikiMarkupRefused),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for WikiSaveRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiSaveRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiSaveRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
