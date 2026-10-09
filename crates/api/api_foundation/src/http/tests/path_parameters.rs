//! Unit coverage for [`PathParams`] and [`ApiError::from_path_rejection`].
//!
//! **Role:** drives axum's real `Path` rejections through the extractor behind real routes, so
//! each case exercises the rejection a handler receives in production.
//!
//! **Position:** sibling test module of `api_foundation::http::path_parameters`.
//!
//! **Signals & state:** none; each case builds its own router.
//!
//! **Invariants:** a decodable segment reaches the handler; an undecodable one answers `400` in
//! the `{error}` envelope naming the parameter; an extractor that does not match its route
//! answers `500 internal error`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::response::Json;
use axum::routing::get;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use super::*;

/// Echoes the decoded id.
async fn one_id(PathParams(id): PathParams<Uuid>) -> Json<Value> {
    Json(json!({ "id": id }))
}

/// Echoes both decoded ids; mounted on a one-parameter route it cannot match that route.
async fn two_ids(PathParams((first, second)): PathParams<(Uuid, Uuid)>) -> Json<Value> {
    Json(json!({ "first": first, "second": second }))
}

/// Requests `uri` and returns the status, the content type and the decoded JSON answer.
async fn get_probe(uri: &str) -> (StatusCode, String, Value) {
    let router = Router::new()
        .route("/items/{itemId}", get(one_id))
        .route("/mismatched/{itemId}", get(two_ids));
    let response = router
        .oneshot(Request::get(uri).body(Body::empty()).expect("request"))
        .await
        .expect("the router answers");
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let body = serde_json::from_slice(&bytes).unwrap_or_else(|_| {
        panic!(
            "the answer to {uri} is not JSON: {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, content_type, body)
}

#[tokio::test]
async fn undecodable_path_parameter_answers_400_in_the_envelope() {
    let (status, content_type, body) = get_probe("/items/not-a-uuid").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(content_type, "application/json");
    let message = body["error"].as_str().expect("`error` is a string");
    assert!(
        message.starts_with("invalid path parameter: "),
        "the message names the path: {message}"
    );
    assert!(
        message.contains("itemId"),
        "the message names the refused parameter: {message}"
    );
    assert_eq!(
        body.as_object().map(|envelope| envelope.len()),
        Some(1),
        "the envelope carries `error` alone: {body}"
    );
}
