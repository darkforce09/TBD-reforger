// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/content-upload.schema.json — regenerate with: cargo xtask ci schema-codegen

///The details of a content request refused before its handler reads it, or by the upload store: request_too_large (413) for a JSON body over the request limit, storage_unavailable (503) when the upload store fails. A missing or wrong JSON content type answers 415, and any other unreadable JSON body 400 with the reason as its error.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ContentRefusal {
    pub code: ContentRefusalCode,
}
///`ContentRefusalCode`
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
pub enum ContentRefusalCode {
    #[serde(rename = "request_too_large")]
    RequestTooLarge,
    #[serde(rename = "storage_unavailable")]
    StorageUnavailable,
}
impl ::std::fmt::Display for ContentRefusalCode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::RequestTooLarge => f.write_str("request_too_large"),
            Self::StorageUnavailable => f.write_str("storage_unavailable"),
        }
    }
}
impl ::std::str::FromStr for ContentRefusalCode {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "request_too_large" => Ok(Self::RequestTooLarge),
            "storage_unavailable" => Ok(Self::StorageUnavailable),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ContentRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ContentRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ContentRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
