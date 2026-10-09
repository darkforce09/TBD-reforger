//! Unit coverage for [`ApiError::from_json_rejection`], [`ApiError::from_query_rejection`] and
//! the error envelope.
//!
//! The rejections come from axum's real `Json` extractor behind a real body limit and its real
//! `Query` extractor, so each case exercises the rejection a handler receives in production
//! rather than a hand-built stand-in.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{DefaultBodyLimit, Query};
use axum::http::{Request, header};
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::Value;
use tower::ServiceExt;

use super::*;

/// The body limit of the probe route: small enough that an oversized body costs nothing.
const PROBE_BODY_LIMIT: usize = 64;

/// The body the probe route decodes.
#[derive(Debug, Deserialize)]
struct Probe {
    name: String,
}

/// Echoes the decoded name, or answers the mapped rejection.
async fn probe(body: Result<Json<Probe>, JsonRejection>) -> Result<Json<Value>, ApiError> {
    let Json(probe) = body.map_err(ApiError::from_json_rejection)?;
    Ok(Json(json!({ "name": probe.name })))
}

/// Posts `body` to the probe route and returns the status with the decoded JSON answer.
async fn post_probe(content_type: Option<&str>, body: impl Into<Body>) -> (StatusCode, Value) {
    let router = Router::new()
        .route("/probe", post(probe))
        .layer(DefaultBodyLimit::max(PROBE_BODY_LIMIT));
    let mut request = Request::post("/probe");
    if let Some(content_type) = content_type {
        request = request.header(header::CONTENT_TYPE, content_type);
    }
    let response = router
        .oneshot(request.body(body.into()).expect("request"))
        .await
        .expect("the router answers");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    (status, serde_json::from_slice(&bytes).expect("JSON answer"))
}

/// The query string the page probe route decodes.
#[derive(Debug, Deserialize)]
struct PageProbe {
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Echoes the decoded page window, or answers the mapped rejection.
async fn page_probe(
    query: Result<Query<PageProbe>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let Query(page) =
        query.map_err(|rejection| ApiError::from_query_rejection(rejection, "probe page query"))?;
    Ok(Json(json!({ "limit": page.limit, "offset": page.offset })))
}

/// Requests `uri` from the page probe route and returns the status with the decoded JSON answer.
async fn get_page_probe(uri: &str) -> (StatusCode, Value) {
    let router = Router::new().route("/page", get(page_probe));
    let response = router
        .oneshot(Request::get(uri).body(Body::empty()).expect("request"))
        .await
        .expect("the router answers");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    (status, serde_json::from_slice(&bytes).expect("JSON answer"))
}

/// The `error` message of an envelope, which is always a non-empty string.
fn error_message(envelope: &Value) -> &str {
    let message = envelope["error"].as_str().expect("`error` is a string");
    assert!(!message.is_empty(), "`error` is never empty: {envelope}");
    message
}

#[tokio::test]
async fn json_body_over_the_limit_answers_413_request_too_large() {
    let oversized = format!(r#"{{"name":"{}"}}"#, "x".repeat(PROBE_BODY_LIMIT * 4));
    let (status, body) = post_probe(Some("application/json"), oversized).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    error_message(&body);
    assert_eq!(body["details"], json!({ "code": "request_too_large" }));
}

#[tokio::test]
async fn missing_content_type_answers_415() {
    let (status, body) = post_probe(None, r#"{"name":"alpha"}"#).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(error_message(&body).contains("Content-Type"), "{body}");
    assert!(body.get("details").is_none(), "{body}");
}

#[tokio::test]
async fn json_that_does_not_decode_answers_400_not_422() {
    let (status, body) = post_probe(Some("application/json"), r#"{"name":5}"#).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let message = error_message(&body);
    assert!(
        message.starts_with("Failed to deserialize the JSON body into the target type"),
        "{body}"
    );
    assert!(
        message.contains("name"),
        "the reason names the field: {body}"
    );
    assert!(body.get("details").is_none(), "{body}");
}

#[tokio::test]
async fn query_string_that_does_not_decode_answers_400_in_the_envelope_naming_the_parameter() {
    for (uri, parameter) in [
        ("/page?limit=abc", "limit"),
        ("/page?limit=1.5", "limit"),
        ("/page?limit=", "limit"),
        ("/page?limit=5&offset=ten", "offset"),
        ("/page?offset=99999999999999999999", "offset"),
    ] {
        let (status, body) = get_page_probe(uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
        let message = error_message(&body);
        assert!(
            message.starts_with("invalid probe page query: Failed to deserialize query string"),
            "{uri}: the message names the query and carries the rejection's reason: {body}"
        );
        assert!(
            message.contains(parameter),
            "{uri}: the reason names `{parameter}`: {body}"
        );
        assert!(body.get("details").is_none(), "{uri}: {body}");
    }
}
