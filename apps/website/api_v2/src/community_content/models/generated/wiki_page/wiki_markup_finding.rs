// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

///One construct a save refuses. line is the 1-based line of body_md where it starts; code names the rule (an unsafe link URL, an unsafe image URL, raw HTML, or nesting deeper than 16); detail explains it to the author.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WikiMarkupFinding {
    pub code: WikiMarkupFindingCode,
    pub detail: WikiMarkupFindingDetail,
    pub line: ::std::num::NonZeroU64,
}
///`WikiMarkupFindingCode`
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
pub enum WikiMarkupFindingCode {
    #[serde(rename = "unsafe_link_url")]
    UnsafeLinkUrl,
    #[serde(rename = "unsafe_image_url")]
    UnsafeImageUrl,
    #[serde(rename = "raw_html")]
    RawHtml,
    #[serde(rename = "nesting_too_deep")]
    NestingTooDeep,
}
impl ::std::fmt::Display for WikiMarkupFindingCode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::UnsafeLinkUrl => f.write_str("unsafe_link_url"),
            Self::UnsafeImageUrl => f.write_str("unsafe_image_url"),
            Self::RawHtml => f.write_str("raw_html"),
            Self::NestingTooDeep => f.write_str("nesting_too_deep"),
        }
    }
}
impl ::std::str::FromStr for WikiMarkupFindingCode {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "unsafe_link_url" => Ok(Self::UnsafeLinkUrl),
            "unsafe_image_url" => Ok(Self::UnsafeImageUrl),
            "raw_html" => Ok(Self::RawHtml),
            "nesting_too_deep" => Ok(Self::NestingTooDeep),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for WikiMarkupFindingCode {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiMarkupFindingCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiMarkupFindingCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`WikiMarkupFindingDetail`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct WikiMarkupFindingDetail(::std::string::String);
impl ::std::ops::Deref for WikiMarkupFindingDetail {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<WikiMarkupFindingDetail> for ::std::string::String {
    fn from(value: WikiMarkupFindingDetail) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for WikiMarkupFindingDetail {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for WikiMarkupFindingDetail {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiMarkupFindingDetail {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiMarkupFindingDetail {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for WikiMarkupFindingDetail {
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
