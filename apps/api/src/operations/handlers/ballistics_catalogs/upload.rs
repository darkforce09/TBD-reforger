//! The administrator's catalog upload.
//!
//! **Role:** answers `POST /api/v1/ballistics-catalogs`: reads the multipart parts `catalog` and
//! `calibration`, decodes them, refuses a duplicate, runs the calibration and stores an accepted
//! version with its audit line, answering the whole upload report.
//!
//! **Position:** registered by [`crate::operations::routes::routes`] behind the body limit
//! [`MAX_CATALOG_UPLOAD_BODY_BYTES`]; judges through
//! [`crate::operations::services::ballistics_catalogs::upload_validation`] and stores through
//! [`crate::operations::services::ballistics_catalogs::catalog_store`]. The administrator
//! catalogs page is the caller.
//!
//! **Signals & state:** none; at most the two parts' bytes in memory.
//!
//! **Invariants:**
//! - Refusals, in the order they are checked: a caller below administrator 401/403 before the
//!   body is read; a request that is not `multipart/form-data` 415; a body over the route limit
//!   or a part over its own cap 413 with `details.code = "request_too_large"`; a part declaring a
//!   content type other than JSON or octet-stream 415; a missing or repeated part 400
//!   (`missing_part`, `repeated_part`); a part that does not decode, or a catalog identity field
//!   that breaks its pattern, 400 with the refusal's code; a taken version or taken bytes 409
//!   (`catalog_version_exists`, `catalog_bytes_exist`); a calibration with any failure 422 with
//!   the whole report under `details`.
//! - An accepted pair answers 201 with the whole report; the version and its audit line are
//!   stored in one transaction, so a refused upload stores nothing.
//! - The calibration runs on the blocking pool, never on the async executor.
//! - The part caps hold the committed vanilla pair (a 10.6 kB catalog and a 6.4 MB calibration
//!   bundle) more than twice over.
//!
//! @contract ballistics-catalog.schema.json#/definitions/CatalogUploadReport

use axum::extract::multipart::{Field, MultipartError, MultipartRejection};
use axum::extract::{Multipart, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Json;
use serde_json::json;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::operations::services::ballistics_catalogs::catalog_store::{
    CatalogDuplicate, NewCatalogVersion, StoreRefusal, find_duplicate, store_catalog_version,
};
use crate::operations::services::ballistics_catalogs::upload_validation::{
    CatalogUploadReport, DecodedUpload, decode_upload, judge_upload,
};

/// The largest accepted `catalog` part, in bytes (1 MiB).
pub const MAX_CATALOG_PART_BYTES: usize = 1 << 20;
/// The largest accepted `calibration` part, in bytes (16 MiB).
pub const MAX_CALIBRATION_PART_BYTES: usize = 16 << 20;
/// Room for the multipart boundaries and part headers (64 KiB).
const MULTIPART_FRAMING_BYTES: usize = 64 << 10;
/// The route's body limit: both parts at their caps plus the framing.
pub const MAX_CATALOG_UPLOAD_BODY_BYTES: usize =
    MAX_CATALOG_PART_BYTES + MAX_CALIBRATION_PART_BYTES + MULTIPART_FRAMING_BYTES;

/// The multipart part that carries the catalog.
const CATALOG_PART: &str = "catalog";
/// The multipart part that carries the calibration bundle.
const CALIBRATION_PART: &str = "calibration";
/// The `details.code` of a body or part over its limit.
const REQUEST_TOO_LARGE_CODE: &str = "request_too_large";

/// `POST /api/v1/ballistics-catalogs`: judges and stores one catalog version.
///
/// @route POST /api/v1/ballistics-catalogs
pub async fn upload_catalog(
    State(state): State<AppState>,
    AdminUser(administrator): AdminUser,
    headers: HeaderMap,
    multipart: Result<Multipart, MultipartRejection>,
) -> Result<(StatusCode, Json<CatalogUploadReport>), ApiError> {
    let mut multipart = multipart.map_err(|rejection| multipart_rejection(&headers, rejection))?;
    let (catalog_bytes, calibration_bytes) = read_parts(&mut multipart).await?;
    let upload = decode_upload(&catalog_bytes, &calibration_bytes).map_err(|refusal| {
        ApiError::with_details(
            StatusCode::BAD_REQUEST,
            refusal.message(),
            json!({ "code": refusal.code() }),
        )
    })?;
    let catalog = upload.catalog();
    if let Some(duplicate) = find_duplicate(
        &state.pool,
        catalog.catalog_id.as_str(),
        upload.catalog_version,
        &upload.pinned.sha256,
    )
    .await?
    {
        return Err(duplicate_refusal(duplicate));
    }

    let (upload, report) = tokio::task::spawn_blocking(move || {
        let report = judge_upload(&upload);
        (upload, report)
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "catalog calibration task failed");
        ApiError::internal("internal error")
    })?;
    if !report.accepted {
        return Err(calibration_refusal(&report));
    }

    store(
        &state,
        &upload,
        &catalog_bytes,
        &calibration_bytes,
        &report,
        &administrator.discord_id,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(report)))
}

/// Stores the accepted pair as text; both parts decoded as JSON, so both are UTF-8.
async fn store(
    state: &AppState,
    upload: &DecodedUpload,
    catalog_bytes: &[u8],
    calibration_bytes: &[u8],
    report: &CatalogUploadReport,
    uploaded_by: &str,
) -> Result<(), ApiError> {
    let text = |bytes: &[u8]| {
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| ApiError::bad_request("the upload parts must be UTF-8 JSON"))
    };
    let catalog_json = text(catalog_bytes)?;
    let calibration_json = text(calibration_bytes)?;
    let stored = store_catalog_version(
        &state.pool,
        NewCatalogVersion {
            upload,
            catalog_json: &catalog_json,
            calibration_json: &calibration_json,
            report,
            uploaded_by,
        },
    )
    .await;
    match stored {
        Ok(_) => Ok(()),
        Err(StoreRefusal::Duplicate(duplicate)) => Err(duplicate_refusal(duplicate)),
        Err(StoreRefusal::Database(error)) => Err(error.into()),
    }
}

/// The two parts' bytes; every other part is skipped.
async fn read_parts(multipart: &mut Multipart) -> Result<(Vec<u8>, Vec<u8>), ApiError> {
    let mut catalog = None;
    let mut calibration = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        let (slot, cap, name) = match field.name() {
            Some(CATALOG_PART) => (&mut catalog, MAX_CATALOG_PART_BYTES, CATALOG_PART),
            Some(CALIBRATION_PART) => (
                &mut calibration,
                MAX_CALIBRATION_PART_BYTES,
                CALIBRATION_PART,
            ),
            _ => continue,
        };
        if slot.is_some() {
            return Err(part_refusal(
                "repeated_part",
                format!("the multipart body carries the {name} part twice"),
            ));
        }
        check_part_content_type(&field, name)?;
        *slot = Some(read_capped(field, cap).await?);
    }
    match (catalog, calibration) {
        (Some(catalog), Some(calibration)) => Ok((catalog, calibration)),
        (catalog, _) => {
            let missing = if catalog.is_none() {
                CATALOG_PART
            } else {
                CALIBRATION_PART
            };
            Err(part_refusal(
                "missing_part",
                format!("the multipart body has no {missing} part"),
            ))
        }
    }
}

/// Refuses a part that declares a content type other than JSON or octet-stream with 415.
fn check_part_content_type(field: &Field<'_>, name: &str) -> Result<(), ApiError> {
    let Some(content_type) = field.content_type() else {
        return Ok(());
    };
    let essence = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if essence == "application/json" || essence == "application/octet-stream" {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::UNSUPPORTED_MEDIA_TYPE,
        format!("the {name} part must be application/json, not {content_type}"),
    ))
}

/// The part's bytes, refused with 413 as soon as they pass `cap`.
async fn read_capped(mut field: Field<'_>, cap: usize) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        if bytes.len() + chunk.len() > cap {
            return Err(too_large(format!(
                "the {} part is larger than {} MiB",
                field.name().unwrap_or("upload"),
                cap >> 20
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// A request the multipart extractor refuses: 415 when it is not `multipart/form-data`, else the
/// rejection's own status.
fn multipart_rejection(headers: &HeaderMap, rejection: MultipartRejection) -> ApiError {
    let is_multipart = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("multipart/form-data")
        });
    if !is_multipart {
        return ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "the upload must be multipart/form-data with the parts catalog and calibration",
        );
    }
    ApiError::new(rejection.status(), rejection.body_text())
}

/// A multipart read failure: over the body limit 413, anything else its own status and text.
fn multipart_error(error: MultipartError) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return too_large(format!(
            "the upload is larger than {} bytes",
            MAX_CATALOG_UPLOAD_BODY_BYTES
        ));
    }
    ApiError::new(error.status(), error.body_text())
}

fn too_large(message: String) -> ApiError {
    ApiError::with_details(
        StatusCode::PAYLOAD_TOO_LARGE,
        message,
        json!({ "code": REQUEST_TOO_LARGE_CODE }),
    )
}

fn part_refusal(code: &str, message: String) -> ApiError {
    ApiError::with_details(StatusCode::BAD_REQUEST, message, json!({ "code": code }))
}

fn duplicate_refusal(duplicate: CatalogDuplicate) -> ApiError {
    let message = match duplicate {
        CatalogDuplicate::VersionTaken => "this catalog version is already stored",
        CatalogDuplicate::BytesTaken => {
            "a version of this catalog with the same bytes is already stored"
        }
    };
    ApiError::with_details(
        StatusCode::CONFLICT,
        message,
        json!({ "code": duplicate.code() }),
    )
}

/// The 422 answer: the whole report under `details`.
fn calibration_refusal(report: &CatalogUploadReport) -> ApiError {
    let details = serde_json::to_value(report).unwrap_or_else(|error| {
        tracing::error!(%error, "catalog upload report does not serialise");
        json!({ "accepted": false })
    });
    ApiError::with_details(
        StatusCode::UNPROCESSABLE_ENTITY,
        format!(
            "the calibration bundle refuses the catalog: {} failures",
            report.calibration.failures.len()
        ),
        details,
    )
}
