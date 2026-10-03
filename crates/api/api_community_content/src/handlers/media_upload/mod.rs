//! The content manager's image upload.
//!
//! **Role:** answers `POST /api/v1/cms/uploads`: reads the multipart `file` field, accepts a JPEG,
//! PNG or WebP image of at most [`MAX_UPLOAD_BYTES`], stores it under a random name in the upload
//! directory and answers the URL it is served at, 201 `{url}`.
//! **Position:** registered by [`crate::routes::routes`] behind the multipart
//! body limit `api_http_layer::middleware::MAX_MULTIPART_BODY`; checks the file with `image_format` and
//! writes it with `upload_store` into `Config::upload_dir`, which the API's router serves at
//! `/uploads`.
//! **Signals & state:** the upload directory on disk; nothing in memory beyond one file's bytes.
//! **Invariants:**
//! - Refusals, in the order they are checked: a body that is not multipart or cannot be read
//!   answers axum's own status (400) and a body over the limit 413 with
//!   `details.code = "request_too_large"`, never 400; a body with no `file` field 400; an extension
//!   other than `jpg`, `jpeg`, `png` or `webp` 415 before the bytes are read; a file over
//!   [`MAX_UPLOAD_BYTES`] 413; leading bytes that are not the extension's format 415.
//! - Nothing is written until the file passes every check, and the write is a staging file plus
//!   an atomic rename, so no partial file is ever served.
//! - A storage failure is logged with its io error and answers 503 with
//!   `details.code = "storage_unavailable"`; the error text never reaches the client.
//! - The answered `url` is `/uploads/<uuid-v4>.<extension>`, checked against the contract pattern
//!   of [`UploadResponseUrl`] before it is sent.

mod image_format;
mod upload_store;

use std::path::Path;

use axum::extract::multipart::{Field, MultipartError, MultipartRejection};
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::json;
use uuid::Uuid;

use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;
use contract_schema_types::community_content::content_upload::{
    ContentRefusalCode, UploadResponse, UploadResponseUrl,
};

use image_format::UploadImageExtension;
use upload_store::store_upload;

/// The largest accepted image, in bytes (5 MiB). The route's multipart body limit leaves room
/// above it for the multipart framing.
pub const MAX_UPLOAD_BYTES: usize = 5 << 20;
/// The multipart field that carries the image.
const FILE_FIELD: &str = "file";
/// The path prefix the API's router serves the upload directory at.
const UPLOADS_URL_PREFIX: &str = "/uploads/";

/// `POST /api/v1/cms/uploads`: stores one image and answers its URL with 201.
///
/// @route POST /api/v1/cms/uploads
pub async fn upload_image(
    State(state): State<AppState>,
    _administrator: AdminUser,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<UploadResponse>), ApiError> {
    let mut multipart =
        multipart.map_err(|rejection| ApiError::new(rejection.status(), rejection.body_text()))?;
    let (extension, bytes) = read_image_field(&mut multipart).await?;

    let file_name = format!("{}.{}", Uuid::new_v4(), extension.as_str());
    let url = UploadResponseUrl::try_from(format!("{UPLOADS_URL_PREFIX}{file_name}")).map_err(
        |error| {
            tracing::error!(%error, %file_name, "upload URL does not match the upload contract");
            ApiError::internal("internal error")
        },
    )?;
    let directory = Path::new(&state.cfg.upload_dir);
    store_upload(directory, &file_name, &bytes)
        .await
        .map_err(|error| {
            tracing::error!(
                %error,
                directory = %directory.display(),
                %file_name,
                "upload store failed"
            );
            ApiError::with_details(
                StatusCode::SERVICE_UNAVAILABLE,
                "upload storage is unavailable",
                json!({ "code": ContentRefusalCode::StorageUnavailable }),
            )
        })?;
    Ok((StatusCode::CREATED, Json(UploadResponse { url })))
}

/// The first `file` field's extension and bytes, checked; every other field is skipped.
async fn read_image_field(
    multipart: &mut Multipart,
) -> Result<(UploadImageExtension, Vec<u8>), ApiError> {
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() != Some(FILE_FIELD) {
            continue;
        }
        let extension = UploadImageExtension::from_file_name(field.file_name().unwrap_or(""))
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "only .jpg, .jpeg, .png and .webp images are accepted",
                )
            })?;
        let bytes = read_capped(field).await?;
        if !extension.matches_signature(&bytes) {
            return Err(ApiError::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                format!(
                    "the file's content is not a {} image",
                    extension.format_name()
                ),
            ));
        }
        return Ok((extension, bytes));
    }
    Err(ApiError::bad_request(
        "the multipart body has no file field",
    ))
}

/// The field's bytes, refused with 413 as soon as they pass [`MAX_UPLOAD_BYTES`].
async fn read_capped(mut field: Field<'_>) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        if bytes.len() + chunk.len() > MAX_UPLOAD_BYTES {
            return Err(upload_too_large());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// A multipart read failure: over the body limit answers [`upload_too_large`], anything else the
/// failure's own status and text.
fn multipart_error(error: MultipartError) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return upload_too_large();
    }
    ApiError::new(error.status(), error.body_text())
}

/// The 413 answer of a body or file over its limit.
fn upload_too_large() -> ApiError {
    ApiError::with_details(
        StatusCode::PAYLOAD_TOO_LARGE,
        format!("the upload is larger than {} MiB", MAX_UPLOAD_BYTES >> 20),
        json!({ "code": ContentRefusalCode::RequestTooLarge }),
    )
}
