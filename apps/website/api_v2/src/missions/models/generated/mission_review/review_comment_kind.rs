// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

///A thread comment, the feedback of a rejection, or the conditions of an approval.
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
pub enum ReviewCommentKind {
    #[serde(rename = "comment")]
    Comment,
    #[serde(rename = "rejection")]
    Rejection,
    #[serde(rename = "approval_conditions")]
    ApprovalConditions,
}
impl ::std::fmt::Display for ReviewCommentKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Comment => f.write_str("comment"),
            Self::Rejection => f.write_str("rejection"),
            Self::ApprovalConditions => f.write_str("approval_conditions"),
        }
    }
}
impl ::std::str::FromStr for ReviewCommentKind {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "comment" => Ok(Self::Comment),
            "rejection" => Ok(Self::Rejection),
            "approval_conditions" => Ok(Self::ApprovalConditions),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ReviewCommentKind {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReviewCommentKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReviewCommentKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
