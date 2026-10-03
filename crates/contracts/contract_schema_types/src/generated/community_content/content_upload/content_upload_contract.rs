// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/content-upload.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::UploadResponse;

///Content media uploads and the error envelope of the content routes. The root is an accepted upload.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct ContentUploadContract(pub UploadResponse);
impl ::std::ops::Deref for ContentUploadContract {
    type Target = UploadResponse;
    fn deref(&self) -> &UploadResponse {
        &self.0
    }
}
impl ::std::convert::From<ContentUploadContract> for UploadResponse {
    fn from(value: ContentUploadContract) -> Self {
        value.0
    }
}
impl ::std::convert::From<UploadResponse> for ContentUploadContract {
    fn from(value: UploadResponse) -> Self {
        Self(value)
    }
}
