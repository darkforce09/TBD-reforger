// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::ReviewCommentKind;

///One comment of the review thread, with the review, version and artifact it concerns.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ReviewComment {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub artifact_id: ::std::option::Option<::uuid::Uuid>,
    pub author_id: ReviewCommentAuthorId,
    pub author_name: ::std::string::String,
    ///Trimmed; at most 8000 bytes of UTF-8.
    pub body: ReviewCommentBody,
    pub created_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub id: ::uuid::Uuid,
    pub kind: ReviewCommentKind,
    pub mission_id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub mission_version_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub review_id: ::std::option::Option<::uuid::Uuid>,
}
///`ReviewCommentAuthorId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ReviewCommentAuthorId(::std::string::String);
impl ::std::ops::Deref for ReviewCommentAuthorId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ReviewCommentAuthorId> for ::std::string::String {
    fn from(value: ReviewCommentAuthorId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ReviewCommentAuthorId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ReviewCommentAuthorId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReviewCommentAuthorId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReviewCommentAuthorId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ReviewCommentAuthorId {
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
///Trimmed; at most 8000 bytes of UTF-8.
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ReviewCommentBody(::std::string::String);
impl ::std::ops::Deref for ReviewCommentBody {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ReviewCommentBody> for ::std::string::String {
    fn from(value: ReviewCommentBody) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ReviewCommentBody {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 8000usize {
            return Err("longer than 8000 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("\\S").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"\\S\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ReviewCommentBody {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReviewCommentBody {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReviewCommentBody {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ReviewCommentBody {
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
