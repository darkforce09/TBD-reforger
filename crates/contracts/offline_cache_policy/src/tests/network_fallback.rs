//! Tests for [`super`] — when the saved copy answers.

use super::*;

#[test]
fn gateway_and_server_failures_prefer_the_saved_copy() {
    for status in [500, 502, 503, 504, 520, 599] {
        assert!(is_gateway_or_server_failure(status), "{status}");
        assert!(
            prefers_saved_copy(NetworkAnswer::Status(status)),
            "a {status} from a proxy whose API is down must be answered from the saved copy"
        );
    }
}

#[test]
fn an_unreachable_network_prefers_the_saved_copy() {
    assert!(prefers_saved_copy(NetworkAnswer::Unreachable));
}

#[test]
fn successes_redirects_and_refusals_are_never_masked() {
    for status in [
        0, 200, 204, 206, 301, 304, 400, 401, 403, 404, 409, 416, 429, 499, 600,
    ] {
        assert!(
            !prefers_saved_copy(NetworkAnswer::Status(status)),
            "{status} is the server's real answer"
        );
    }
    assert!(!is_gateway_or_server_failure(499));
    assert!(!is_gateway_or_server_failure(600));
}
