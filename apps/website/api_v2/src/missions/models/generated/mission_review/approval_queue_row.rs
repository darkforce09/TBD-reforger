// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::TerrainType;

///One mission awaiting review. The review fields are absent for a mission submitted before artifacts existed; it is decided only after its author resubmits it.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ApprovalQueueRow {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub artifact_digest: ::std::option::Option<ApprovalQueueRowArtifactDigest>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub artifact_id: ::std::option::Option<::uuid::Uuid>,
    pub author_id: ::std::string::String,
    pub author_name: ::std::string::String,
    pub mission_id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub review_id: ::std::option::Option<::uuid::Uuid>,
    pub submitted_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub terrain: TerrainType,
    pub title: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub version_semver: ::std::option::Option<ApprovalQueueRowVersionSemver>,
}
///`ApprovalQueueRowArtifactDigest`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ApprovalQueueRowArtifactDigest(::std::string::String);
impl ::std::ops::Deref for ApprovalQueueRowArtifactDigest {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ApprovalQueueRowArtifactDigest> for ::std::string::String {
    fn from(value: ApprovalQueueRowArtifactDigest) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ApprovalQueueRowArtifactDigest {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ApprovalQueueRowArtifactDigest {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ApprovalQueueRowArtifactDigest {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ApprovalQueueRowArtifactDigest {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ApprovalQueueRowArtifactDigest {
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
///`ApprovalQueueRowVersionSemver`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ApprovalQueueRowVersionSemver(::std::string::String);
impl ::std::ops::Deref for ApprovalQueueRowVersionSemver {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ApprovalQueueRowVersionSemver> for ::std::string::String {
    fn from(value: ApprovalQueueRowVersionSemver) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ApprovalQueueRowVersionSemver {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ApprovalQueueRowVersionSemver {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ApprovalQueueRowVersionSemver {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ApprovalQueueRowVersionSemver {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ApprovalQueueRowVersionSemver {
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
