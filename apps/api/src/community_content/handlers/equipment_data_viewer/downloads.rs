//! Stream complete original documents selected through the validated index.
use super::{Failure, failure, viewer_query};
use crate::{
    community_content::services::equipment_data_viewer::{queries::ViewerQuery, source::manifest},
    core::application_state::AppState,
};
use axum::{
    body::Body,
    extract::{Query, State, rejection::QueryRejection},
    http::{Response, header},
};
use tokio::io::AsyncReadExt;

/// @route GET /api/v1/debug/equipment-data/download
pub async fn download(
    State(state): State<AppState>,
    query: Result<Query<ViewerQuery>, QueryRejection>,
) -> Result<Response<Body>, Failure> {
    let p = viewer_query(query)?;
    let d = state
        .equipment_data
        .select(p.dataset.as_deref())
        .map_err(failure)?
        .dataset(p.generation())
        .await
        .map_err(failure)?;
    let kind = p.document.as_deref().unwrap_or("generation");
    let file = match kind {
        "generation"
        | "manifest"
        | "field_definitions"
        | "native_types"
        | "selection_report"
        | "resource_index"
        | "publication_receipt" => format!("{kind}.json"),
        "record" | "source" => {
            let statement = if kind == "record" {
                "SELECT record_file FROM resources WHERE resource_id=?"
            } else {
                "SELECT source_file FROM resources WHERE resource_id=?"
            };
            sqlx::query_scalar::<_, String>(statement)
                .bind(p.resource().map_err(failure)?)
                .fetch_optional(&d.pool)
                .await
                .map_err(failure)?
                .ok_or_else(|| failure("resource not found"))?
        }
        _ => return Err(failure("unknown document kind")),
    };
    let path = manifest::safe_child(&d.root, &file).map_err(failure)?;
    let mut file = tokio::fs::File::open(path).await.map_err(failure)?;
    let size = file.metadata().await.map_err(failure)?.len();
    let stream = async_stream::try_stream! {
        let mut buffer=vec![0u8;65536];
        loop {let read=file.read(&mut buffer).await?;if read==0{break;}yield axum::body::Bytes::copy_from_slice(&buffer[..read]);}
    };
    let stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<axum::body::Bytes, std::io::Error>> + Send>,
    > = Box::pin(stream);
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CONTENT_LENGTH, size)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{kind}.json\""),
        )
        .header(header::CACHE_CONTROL, "private, no-store")
        .body(Body::from_stream(stream))
        .map_err(failure)
}
