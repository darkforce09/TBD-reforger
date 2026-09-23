//! The refusal reader: a refused answer keeps its status, its sentence and its structured reason.

use super::*;
use serde_json::json;

/// A coded refusal as the claim routes send it: the sentence, and the reason with its fields.
#[test]
fn a_coded_refusal_keeps_its_reason_and_its_fields() {
    let body = json!({
        "error": "guest reservations open at 2026-08-01T12:00:00Z",
        "details": {
            "code": "QUOTA_NOT_OPEN",
            "quota_kind": "guest",
            "opens_at": "2026-08-01T12:00:00Z"
        }
    });
    let refusal = ApiRefusal::from_error_body(409, Some(&body));
    assert_eq!(refusal.status, 409);
    assert_eq!(refusal.code(), Some("QUOTA_NOT_OPEN"));
    assert_eq!(refusal.detail("quota_kind"), Some("guest"));
    assert_eq!(refusal.detail("opens_at"), Some("2026-08-01T12:00:00Z"));
    assert_eq!(
        refusal.message_or("fallback"),
        "Guest reservations open at 2026-08-01T12:00:00Z"
    );
}

/// A findings array is prose, not a reason: it is folded into the message the way every other
/// request folds it, and no code is invented from it.
#[test]
fn a_findings_array_is_folded_into_the_message_and_names_no_reason() {
    let body = json!({"error": "invalid body", "details": ["name: empty", "source: missing"]});
    let refusal = ApiRefusal::from_error_body(400, Some(&body));
    assert_eq!(refusal.details, None);
    assert_eq!(refusal.code(), None);
    assert_eq!(
        refusal.message.as_deref(),
        Some("invalid body\nname: empty\nsource: missing")
    );
}

/// A reason field that is not text reads as absent rather than as a stringified number.
#[test]
fn a_non_text_detail_reads_as_absent() {
    let body = json!({
        "error": "access settings changed since this form was loaded",
        "details": {"code": "ACCESS_REVISION_CONFLICT", "access_revision": 7}
    });
    let refusal = ApiRefusal::from_error_body(409, Some(&body));
    assert_eq!(refusal.code(), Some("ACCESS_REVISION_CONFLICT"));
    assert_eq!(refusal.detail("access_revision"), None);
}

/// An unparseable body still yields the status, so the caller can tell a conflict from a refusal.
#[test]
fn an_unreadable_body_keeps_the_status_and_falls_back() {
    let refusal = ApiRefusal::from_error_body(404, None);
    assert_eq!(refusal.status, 404);
    assert_eq!(refusal.code(), None);
    assert_eq!(refusal.message_or("Not found"), "Not found");
}

/// A 2xx answer decodes into the asked-for type; one that is not that shape is the unreadable
/// answer, status `0`, never a success with default values.
#[test]
fn a_success_decodes_or_reports_an_unreadable_answer() {
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Promoted {
        promoted: Vec<String>,
    }
    let ok: Result<Promoted, ApiRefusal> =
        decode_answer(200, Some(json!({"promoted": ["a", "b"]})));
    assert_eq!(
        ok,
        Ok(Promoted {
            promoted: vec!["a".into(), "b".into()]
        })
    );
    let wrong: Result<Promoted, ApiRefusal> = decode_answer(200, Some(json!({"other": 1})));
    assert_eq!(wrong, Err(ApiRefusal::unreadable()));
    let empty: Result<Promoted, ApiRefusal> = decode_answer(201, None);
    assert_eq!(empty, Err(ApiRefusal::unreadable()));
}

/// Any non-2xx status decodes into its refusal, whatever its body.
#[test]
fn a_refused_answer_decodes_into_its_refusal() {
    let refused: Result<serde_json::Value, ApiRefusal> = decode_answer(
        409,
        Some(json!({"error": "this operation is full", "details": {"code": "EVENT_FULL"}})),
    );
    let refusal = refused.unwrap_err();
    assert_eq!(refusal.status, 409);
    assert_eq!(refusal.code(), Some("EVENT_FULL"));
}

/// The plain failure pair converts without inventing a reason, and a terminal `401` stays the
/// session-over signal.
#[test]
fn the_plain_failure_pair_converts_without_a_reason() {
    let expired = ApiRefusal::from((401u16, Some("expired".to_string())));
    assert!(expired.is_session_expired());
    assert_eq!(expired.code(), None);
    let transport = ApiRefusal::from((0u16, None));
    assert_eq!(transport, ApiRefusal::unreadable());
    assert!(!transport.is_session_expired());
}

/// The request helper's refusal arm reads the answer only for a non-`401` status: a `401` must
/// stay a failure, or the refresh-and-retry contract would never see it.
#[test]
fn the_refusal_arm_leaves_a_401_to_the_refresh_contract() {
    let client = crate::v2::core::test_support::class_r_scrub::live_code(
        &crate::v2::core::test_support::pins::client_source(),
    );
    let reader =
        crate::v2::core::test_support::class_r_scrub::only_body(&client, "fn refusal_reader(");
    assert!(
        reader.contains("Consume::Answer(keep) if status != 401 => Some(*keep)"),
        "a refused answer is read as data only when it is not a 401"
    );
    let request = crate::v2::core::test_support::class_r_scrub::only_item(
        &client,
        "async fn request_keeping_refusal<",
    );
    assert!(
        request.contains("request(store, method, path, body, Consume::Answer(keep))"),
        "the refusal-keeping verbs share the one request helper and its single flight"
    );
}
