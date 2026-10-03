//! Unit coverage for the `fail_point!` macro at the two kinds of call site: a function returning
//! `ApiError` and one returning `sqlx::Error`. Unarmed, each runs to its end; armed with `Fail`,
//! each returns at the point with the error its conversion builds; and a call site's future is
//! `Send`, as an axum handler needs.

use axum::http::StatusCode;
use serde_json::json;

use crate::{FailAction, Failpoint, lock_suite};
use api_foundation::error_handling::api_error::ApiError;

/// What a call site returns when it runs past its failpoint.
const PASSED: &str = "passed the failpoint";

/// A handler-shaped call site.
async fn api_error_call_site() -> Result<&'static str, ApiError> {
    fail_point!(RegistryUnitTestFirst);
    Ok(PASSED)
}

/// A service-shaped call site.
async fn sqlx_error_call_site() -> Result<&'static str, sqlx::Error> {
    fail_point!(RegistryUnitTestSecond);
    Ok(PASSED)
}

#[tokio::test]
async fn failpoints_macro_returns_an_internal_api_error_when_armed() {
    let suite = lock_suite().await;
    let unarmed = tokio::spawn(api_error_call_site())
        .await
        .expect("the call site joins");
    assert_eq!(unarmed.expect("an unarmed point passes"), PASSED);

    let _guard = suite.arm(Failpoint::RegistryUnitTestFirst, FailAction::Fail);
    let error = api_error_call_site()
        .await
        .expect_err("an armed point returns at the point");
    assert_eq!(error.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(error.message, "internal error");
    assert_eq!(
        error.details,
        Some(json!({ "failpoint": "RegistryUnitTestFirst" }))
    );
}

#[tokio::test]
async fn failpoints_macro_returns_a_sqlx_protocol_error_when_armed() {
    let suite = lock_suite().await;
    let unarmed = tokio::spawn(sqlx_error_call_site())
        .await
        .expect("the call site joins");
    assert_eq!(unarmed.expect("an unarmed point passes"), PASSED);

    let _guard = suite.arm(Failpoint::RegistryUnitTestSecond, FailAction::Fail);
    let error = sqlx_error_call_site()
        .await
        .expect_err("an armed point returns at the point");
    match &error {
        sqlx::Error::Protocol(message) => assert_eq!(
            message,
            "injected failure at failpoint RegistryUnitTestSecond"
        ),
        other => panic!("expected a protocol error, got {other:?}"),
    }
    let rendered = ApiError::from(error);
    assert_eq!(rendered.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(rendered.message, "internal error");
}
