// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///`MedicalRevivedPayload`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MedicalRevivedPayload {
    pub subject_arma_id: MedicalRevivedPayloadSubjectArmaId,
}
///`MedicalRevivedPayloadSubjectArmaId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MedicalRevivedPayloadSubjectArmaId(::std::string::String);
impl ::std::ops::Deref for MedicalRevivedPayloadSubjectArmaId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MedicalRevivedPayloadSubjectArmaId> for ::std::string::String {
    fn from(value: MedicalRevivedPayloadSubjectArmaId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MedicalRevivedPayloadSubjectArmaId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 128usize {
            return Err("longer than 128 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MedicalRevivedPayloadSubjectArmaId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MedicalRevivedPayloadSubjectArmaId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MedicalRevivedPayloadSubjectArmaId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MedicalRevivedPayloadSubjectArmaId {
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
