// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-session.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/game-runtime/sessions body (mod_runtime credential). A runtime that loaded a mission artifact reports it; both fields come together, and the SHA-256 is of the exact document bytes it loaded. An empty body starts a session that runs no artifact. An unknown artifact answers 422 UNKNOWN_ARTIFACT.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSessionStart {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub loaded_artifact_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub loaded_artifact_sha256: ::std::option::Option<RuntimeSessionStartLoadedArtifactSha256>,
}
impl ::std::default::Default for RuntimeSessionStart {
    fn default() -> Self {
        Self {
            loaded_artifact_id: Default::default(),
            loaded_artifact_sha256: Default::default(),
        }
    }
}
///`RuntimeSessionStartLoadedArtifactSha256`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct RuntimeSessionStartLoadedArtifactSha256(::std::string::String);
impl ::std::ops::Deref for RuntimeSessionStartLoadedArtifactSha256 {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<RuntimeSessionStartLoadedArtifactSha256> for ::std::string::String {
    fn from(value: RuntimeSessionStartLoadedArtifactSha256) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for RuntimeSessionStartLoadedArtifactSha256 {
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
impl ::std::convert::TryFrom<&str> for RuntimeSessionStartLoadedArtifactSha256 {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RuntimeSessionStartLoadedArtifactSha256 {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RuntimeSessionStartLoadedArtifactSha256 {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for RuntimeSessionStartLoadedArtifactSha256 {
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
