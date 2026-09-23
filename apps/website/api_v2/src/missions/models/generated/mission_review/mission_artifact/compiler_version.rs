// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

///`MissionArtifactCompilerVersion`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MissionArtifactCompilerVersion(::std::string::String);
impl ::std::ops::Deref for MissionArtifactCompilerVersion {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MissionArtifactCompilerVersion> for ::std::string::String {
    fn from(value: MissionArtifactCompilerVersion) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MissionArtifactCompilerVersion {
    type Err = super::super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MissionArtifactCompilerVersion {
    type Error = super::super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MissionArtifactCompilerVersion {
    type Error = super::super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MissionArtifactCompilerVersion {
    type Error = super::super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MissionArtifactCompilerVersion {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: super::super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
