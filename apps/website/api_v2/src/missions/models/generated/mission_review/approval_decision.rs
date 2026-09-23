// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/approvals/:id/approve body (administrator): the artifact under review and any conditions. Answers the decided MissionRow; a different artifact answers 409 REVIEWED_ARTIFACT_CHANGED naming the artifact under review.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecision {
    pub artifact_id: ::uuid::Uuid,
    ///Trimmed; a blank value approves without conditions. At most 8000 bytes of UTF-8.
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub conditions: ::std::option::Option<ApprovalDecisionConditions>,
}
///Trimmed; a blank value approves without conditions. At most 8000 bytes of UTF-8.
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct ApprovalDecisionConditions(::std::string::String);
impl ::std::ops::Deref for ApprovalDecisionConditions {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<ApprovalDecisionConditions> for ::std::string::String {
    fn from(value: ApprovalDecisionConditions) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for ApprovalDecisionConditions {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 8000usize {
            return Err("longer than 8000 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for ApprovalDecisionConditions {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ApprovalDecisionConditions {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ApprovalDecisionConditions {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for ApprovalDecisionConditions {
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
