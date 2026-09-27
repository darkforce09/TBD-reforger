//! Anonymous, generation-pinned reads through the normal API middleware.
mod dataset;
mod downloads;
mod resources;
mod source_inspection;

use crate::community_content::services::equipment_data_viewer::PAGE_BYTES;
use crate::core::application_state::AppState;
use axum::{Json, Router, http::StatusCode, routing::get};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

type Failure = (StatusCode, Json<Value>);
type ReadResult<T> = Result<Json<T>, Failure>;

fn failure(error: impl std::fmt::Display) -> Failure {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error":error.to_string()})),
    )
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

pub fn routes() -> Router<AppState> {
    Router::new().nest(
        "/debug/equipment-data",
        Router::new()
            .route("/status", get(dataset::status))
            .route("/overview", get(dataset::overview))
            .route("/resources", get(resources::resources))
            .route("/relationships", get(resources::relationships))
            .route("/fields", get(resources::fields))
            .route("/resource-cards", get(source_inspection::resource_cards))
            .route("/selection", get(source_inspection::selection))
            .route("/containers", get(source_inspection::containers))
            .route("/properties", get(source_inspection::properties))
            .route("/values", get(source_inspection::values))
            .route("/documents", get(source_inspection::documents))
            .route("/download", get(downloads::download)),
    )
}
