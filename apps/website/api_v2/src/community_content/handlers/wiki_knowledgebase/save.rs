//! The administrator's wiki save: create a page or save a new revision of it.
//!
//! **Role:** answers `PUT /api/v1/wiki/{slug}`: checks the slug, the body and the markup, then
//! writes the page, its revision row and the audit line in one transaction.
//! **Position:** registered in [`crate::community_content::routes::routes`]; writes
//! `wiki_pages` and `wiki_page_revisions` through [`super::page_store`], appends the audit line
//! through [`crate::administration::services::required_audit::append_actor_audit`], and answers
//! the saved [`WikiArticle`].
//! **Signals & state:** none; one transaction per accepted save.
//! **Invariants:**
//! - The slug matches `^[a-z0-9-]{1,64}$` and is checked before the body is read, so a caller
//!   who addressed the wrong page hears about the page.
//! - A body that does not decode (a missing or unknown field) answers 400, one over the request
//!   limit 413 and one without a JSON content type 415; `category`, `title` and `body_md` are
//!   non-empty, and `body_md` holds at most [`MAX_BODY_BYTES`] bytes (400
//!   `wiki_body_too_large`).
//! - Markup the service refuses answers 422 `wiki_markup_refused` with every finding, before any
//!   row is touched.
//! - `base_revision: null` creates the page (201) and answers 409 when it exists; a number must
//!   equal the current revision (else 409 `wiki_revision_conflict` with `current_revision`) and
//!   saves the next revision (200); a number for a missing page answers 404.
//! - Lock order: the page row `FOR UPDATE`, then the revision insert, then the audit row; any
//!   failure rolls all three back. The editor is the caller, never a body field.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;

use super::page_store;
use crate::administration::services::required_audit::append_actor_audit;
use crate::community_content::models::wiki::{
    WikiArticle, WikiSaveRefusal, WikiSaveRefusalCode, WikiSaveRequest,
};
use crate::community_content::services::wiki_markup::read_markup;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;

/// The largest `body_md` a save accepts, in bytes.
pub const MAX_BODY_BYTES: usize = 262_144;

/// The longest slug a page may have, in bytes.
pub const MAX_SLUG_BYTES: usize = 64;

/// The audit target type of a wiki save.
const AUDIT_TARGET_TYPE: &str = "wiki_page";

/// `PUT /api/v1/wiki/{slug}` — create a page or save its next revision (admin).
///
/// @route PUT /api/v1/wiki/{slug}
pub async fn save_wiki_page(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(slug): Path<String>,
    body: Result<Json<WikiSaveRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<WikiArticle>), ApiError> {
    if !is_valid_slug(&slug) {
        return Err(ApiError::bad_request(
            "slug must be 1 to 64 lowercase letters, digits or hyphens",
        ));
    }
    let Json(request) = body.map_err(ApiError::from_json_rejection)?;
    check_request(&request)?;
    let reading = read_markup(&request.body_md);
    if !reading.findings.is_empty() {
        return Err(refusal(
            StatusCode::UNPROCESSABLE_ENTITY,
            "the page markup holds links, images, HTML or nesting a wiki page cannot carry",
            WikiSaveRefusal {
                code: WikiSaveRefusalCode::WikiMarkupRefused,
                current_revision: None,
                findings: Some(reading.findings),
            },
        ));
    }

    let editor = admin.0.discord_id.as_str();
    let mut transaction = state.pool.begin().await?;
    let locked = page_store::lock_page(&mut *transaction, &slug).await?;
    let (page_id, created) = match (request.base_revision, locked) {
        (None, Some((_, current))) => return Err(revision_conflict(Some(current))),
        (None, None) => {
            match page_store::insert_page(&mut *transaction, &slug, &request, editor).await? {
                Some(page_id) => (page_id, true),
                None => {
                    let current = page_store::current_revision(&mut *transaction, &slug).await?;
                    return Err(revision_conflict(current));
                }
            }
        }
        (Some(_), None) => return Err(ApiError::not_found("wiki page not found")),
        (Some(base), Some((page_id, current))) => {
            if base != i64::from(current) {
                return Err(revision_conflict(Some(current)));
            }
            page_store::update_page(&mut *transaction, page_id, &request, editor).await?;
            (page_id, false)
        }
    };
    page_store::record_current_revision(&mut *transaction, page_id).await?;
    let page = page_store::page_by_slug(&mut *transaction, &slug)
        .await?
        .ok_or_else(|| ApiError::internal("internal error"))?;
    let (action, message) = if created {
        (
            "wiki_page.created",
            format!("Created wiki page {slug} at revision {}", page.revision),
        )
    } else {
        (
            "wiki_page.updated",
            format!("Saved wiki page {slug} as revision {}", page.revision),
        )
    };
    append_actor_audit(
        &mut transaction,
        editor,
        action,
        AUDIT_TARGET_TYPE,
        &slug,
        &message,
    )
    .await?;
    transaction.commit().await?;

    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(WikiArticle::new(page, reading.blocks))))
}

/// Whether `slug` matches `^[a-z0-9-]{1,64}$`.
pub fn is_valid_slug(slug: &str) -> bool {
    (1..=MAX_SLUG_BYTES).contains(&slug.len())
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// The field checks a decoded body must pass before its markup is read.
fn check_request(request: &WikiSaveRequest) -> Result<(), ApiError> {
    if request.category.is_empty() || request.title.is_empty() || request.body_md.is_empty() {
        return Err(ApiError::bad_request(
            "category, title and body_md are required",
        ));
    }
    if request.base_revision.is_some_and(|base| base < 1) {
        return Err(ApiError::bad_request(
            "base_revision must be null or a revision number of 1 or more",
        ));
    }
    if request.body_md.len() > MAX_BODY_BYTES {
        return Err(refusal(
            StatusCode::BAD_REQUEST,
            &format!("body_md is larger than {MAX_BODY_BYTES} bytes"),
            WikiSaveRefusal {
                code: WikiSaveRefusalCode::WikiBodyTooLarge,
                current_revision: None,
                findings: None,
            },
        ));
    }
    Ok(())
}

/// The 409 of a save whose base is not the page's current revision.
fn revision_conflict(current_revision: Option<i32>) -> ApiError {
    refusal(
        StatusCode::CONFLICT,
        "the wiki page has changed since the revision this save started from",
        WikiSaveRefusal {
            code: WikiSaveRefusalCode::WikiRevisionConflict,
            current_revision,
            findings: None,
        },
    )
}

/// A refused save with `details` set to `refusal`.
fn refusal(status: StatusCode, message: &str, refusal: WikiSaveRefusal) -> ApiError {
    ApiError::with_details(
        status,
        message,
        serde_json::to_value(refusal).unwrap_or_default(),
    )
}
