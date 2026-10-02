// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/content-upload.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/cms/uploads (administrator; multipart/form-data with one file field), answered 201. The file is a JPEG, PNG or WebP image: its extension is jpg, jpeg, png or webp and its leading bytes match that format, else 415. A missing file field answers 400 and a body over the upload limit 413. The file is written under a temporary name and renamed into place, so no partial file is ever served; a storage failure answers 503 storage_unavailable. url is the site-relative path the image is served at.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct UploadResponse {
    pub url: UploadResponseUrl,
}
///`UploadResponseUrl`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct UploadResponseUrl(::std::string::String);
impl ::std::ops::Deref for UploadResponseUrl {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<UploadResponseUrl> for ::std::string::String {
    fn from(value: UploadResponseUrl) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for UploadResponseUrl {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> = ::std::sync::LazyLock::new(
            || {
                ::regress::Regex::new(
                    "^/uploads/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\\.(jpg|jpeg|png|webp)$",
                )
                .unwrap()
            },
        );
        if PATTERN.find(value).is_none() {
            return Err(
                "doesn't match pattern \"^/uploads/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\\.(jpg|jpeg|png|webp)$\""
                    .into(),
            );
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for UploadResponseUrl {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for UploadResponseUrl {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for UploadResponseUrl {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for UploadResponseUrl {
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
