//! CMS media uploads: the multipart thumbnail endpoint, writing into the configured upload
//! directory (`Config::upload_dir`, served back at `/uploads`).

use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;

const MAX_UPLOAD_BYTES: usize = 5 << 20;

/// `POST /api/v1/cms/uploads` — thumbnail upload (multipart "file").
///
/// @route POST /api/v1/cms/uploads
pub async fn upload_image(
    State(state): State<AppState>,
    _a: AdminUser,
    mut mp: Multipart,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let upload_dir = state.cfg.upload_dir.as_str();
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("file field required"))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let filename = field.file_name().unwrap_or("").to_string();
        let ext = ext_lower(&filename);
        let data = field
            .bytes()
            .await
            .map_err(|_| ApiError::bad_request("file field required"))?;
        if data.len() > MAX_UPLOAD_BYTES {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "file exceeds 5MB",
            ));
        }
        if !matches!(ext.as_str(), ".jpg" | ".jpeg" | ".png" | ".webp") {
            return Err(ApiError::new(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "only JPG, PNG, WEBP allowed",
            ));
        }
        std::fs::create_dir_all(upload_dir)
            .map_err(|_| ApiError::internal("storage unavailable"))?;
        let name = format!("{}{ext}", Uuid::new_v4());
        std::fs::write(format!("{upload_dir}/{name}"), &data)
            .map_err(|_| ApiError::internal("could not save file"))?;
        return Ok((
            StatusCode::CREATED,
            Json(json!({ "url": format!("/uploads/{name}") })),
        ));
    }
    Err(ApiError::bad_request("file field required"))
}

/// Lowercase file extension including the dot (`.jpg`), or empty.
fn ext_lower(filename: &str) -> String {
    match filename.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() => format!(".{}", ext.to_lowercase()),
        _ => String::new(),
    }
}
