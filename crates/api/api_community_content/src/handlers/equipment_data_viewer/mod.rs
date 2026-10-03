//! The equipment data viewer's debug reads: anonymous, generation-pinned views of the exported
//! equipment datasets, served through the normal API middleware.
//!
//! **Role:** the handlers behind `GET /api/v1/debug/equipment-data/*` — dataset status and
//! overview, resource, relationship and field listings, source inspection and whole-document
//! downloads — and the failure and page-size shaping they answer through.
//! **Position:** [`crate::routes::routes`] registers them only when the
//! configuration reports a development environment, so a production router answers 404; each
//! handler reads through `AppState::equipment_data`, the service in
//! [`api_equipment_datasets`].
//! **Signals & state:** none; the dataset service owns the loaded generations.
//! **Invariants:** every answer but the `download` stream re-reads as its generated contract type
//! and serialises to at most [`PAGE_BYTES`]; a failure the handlers raise answers 400 in the
//! `{error}` envelope, and so does a query string that does not decode (through
//! [`ApiError::from_query_rejection`]).
pub mod dataset;
pub mod downloads;
pub mod resources;
pub mod source_inspection;

use api_equipment_datasets::{PAGE_BYTES, queries::ViewerQuery};
use api_foundation::error_handling::api_error::ApiError;
use axum::Json;
use axum::extract::Query;
use axum::extract::rejection::QueryRejection;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

type Failure = ApiError;
type ReadResult<T> = Result<Json<T>, Failure>;

/// A failure the handlers raise: 400 in the `{error}` envelope carrying the failure's text.
fn failure(error: impl std::fmt::Display) -> Failure {
    ApiError::bad_request(error.to_string())
}

/// The decoded viewer query, or the 400 `{error}` envelope naming the parameter that failed.
fn viewer_query(query: Result<Query<ViewerQuery>, QueryRejection>) -> Result<ViewerQuery, Failure> {
    query.map(|Query(query)| query).map_err(|rejection| {
        ApiError::from_query_rejection(rejection, "equipment data viewer query")
    })
}

fn response<T: DeserializeOwned + Serialize>(
    dataset_kind: String,
    mut value: Value,
) -> ReadResult<T> {
    value["dataset_kind"] = json!(dataset_kind);
    let result: T = serde_json::from_value(value).map_err(failure)?;
    if serde_json::to_vec(&result).map_err(failure)?.len() > PAGE_BYTES {
        return Err(failure(
            "Response exceeds page limit; use document expansion",
        ));
    }
    Ok(Json(result))
}
