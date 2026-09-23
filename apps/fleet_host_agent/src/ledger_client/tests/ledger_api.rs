use serde_json::json;

use super::*;

fn body(value: Value) -> Vec<u8> {
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn a_stale_fencing_token_is_recognised_by_its_code() {
    let stale = body(json!({
        "error": "the claim is no longer current",
        "details": {"code": "STALE_FENCING_TOKEN", "state": "queued"},
    }));
    assert_eq!(
        classify_refusal(StatusCode::CONFLICT, &stale),
        LedgerError::StaleFencingToken
    );
}

#[test]
fn other_conflicts_carry_their_code_and_are_final() {
    let not_claimed = body(json!({
        "error": "the command is executing",
        "details": {"code": "COMMAND_NOT_CLAIMED", "state": "executing"},
    }));
    let error = classify_refusal(StatusCode::CONFLICT, &not_claimed);
    assert_eq!(
        error,
        LedgerError::Conflict {
            code: "COMMAND_NOT_CLAIMED".to_owned(),
            message: "the command is executing".to_owned(),
        }
    );
    assert!(!error.is_transient());
}

#[test]
fn server_errors_timeouts_and_rate_limits_are_transient() {
    for status in [
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::BAD_GATEWAY,
        StatusCode::SERVICE_UNAVAILABLE,
        StatusCode::GATEWAY_TIMEOUT,
        StatusCode::REQUEST_TIMEOUT,
        StatusCode::TOO_MANY_REQUESTS,
    ] {
        let error = classify_refusal(status, b"<html>proxy error</html>");
        assert!(error.is_transient(), "{status}: {error}");
    }
    assert!(LedgerError::Unreachable("connection refused".to_owned()).is_transient());
}

#[test]
fn client_errors_are_final_and_keep_the_api_message() {
    let revoked = body(json!({"error": "machine credential revoked"}));
    let error = classify_refusal(StatusCode::UNAUTHORIZED, &revoked);
    assert_eq!(
        error,
        LedgerError::Refused {
            status: 401,
            message: "machine credential revoked".to_owned(),
        }
    );
    assert!(!error.is_transient());
    let unexplained = classify_refusal(StatusCode::FORBIDDEN, b"");
    assert_eq!(
        unexplained,
        LedgerError::Refused {
            status: 403,
            message: "Forbidden".to_owned(),
        }
    );
    assert!(!LedgerError::StaleFencingToken.is_transient());
    assert!(!LedgerError::UnreadableAnswer("eof".to_owned()).is_transient());
}

#[test]
fn long_api_messages_are_cut() {
    let long = body(json!({"error": "x".repeat(1000)}));
    let LedgerError::Refused { message, .. } = classify_refusal(StatusCode::BAD_REQUEST, &long)
    else {
        panic!("a refusal");
    };
    assert_eq!(message.len(), API_MESSAGE_MAX_CHARS);
}
