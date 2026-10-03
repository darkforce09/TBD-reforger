// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/wiki-page.schema.json — regenerate with: cargo xtask ci schema-codegen

///One entry of a page's revision history. author_id is the Discord id of the editor, absent when unknown; a page's first revision predating the history takes the page's last editor and update time.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct WikiRevisionSummary {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub author_id: ::std::option::Option<WikiRevisionSummaryAuthorId>,
    pub created_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub revision: ::std::num::NonZeroU64,
    pub title: ::std::string::String,
}
///`WikiRevisionSummaryAuthorId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct WikiRevisionSummaryAuthorId(::std::string::String);
impl ::std::ops::Deref for WikiRevisionSummaryAuthorId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<WikiRevisionSummaryAuthorId> for ::std::string::String {
    fn from(value: WikiRevisionSummaryAuthorId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for WikiRevisionSummaryAuthorId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for WikiRevisionSummaryAuthorId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for WikiRevisionSummaryAuthorId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for WikiRevisionSummaryAuthorId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for WikiRevisionSummaryAuthorId {
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
