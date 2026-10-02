//! A refusal and an empty success are different values, and the total reader proves it.

use super::super::errors::{ApiErr, ApiFailure};
use super::Fetched;

/* ═════════════ a 401 is not an empty list ═════════════ */

/// The defect, stated as a value. An empty success and a dead session used to arrive at the render
/// site as the same thing — nothing, then an empty state. They are now different variants, and no
/// combinator on the type merges them.
#[test]
fn an_empty_result_and_a_401_are_different_values() {
    let empty: Fetched<Vec<u8>> = Ok(Vec::new()).into();
    let dead: Fetched<Vec<u8>> = Err((401u16, None)).into();

    assert_ne!(
        empty, dead,
        "an empty list and an expired session must not be the same value — that equality IS \
         the bug, and it is what turned a dead session into an empty page"
    );
    assert_eq!(empty.data(), Some(&Vec::new()));
    assert!(
        !empty.is_session_expired(),
        "a genuinely empty response must never raise the session banner"
    );
    assert!(
        dead.is_session_expired(),
        "a 401 must reach the render site AS a 401 (perturbation: map Err(_) to Data(vec![]))"
    );
    assert_eq!(dead.data(), None, "a 401 carries no data to render");
}

/// The other half: not every failure is a session expiry. Saying "log in again" to someone
/// whose wifi dropped, or who is logged in but lacks the role, is its own wrong answer.
#[test]
fn only_a_terminal_401_counts_as_a_session_expiry() {
    let cases: [(ApiErr, ApiFailure); 4] = [
        (
            (401, Some("invalid or expired token".into())),
            ApiFailure::SessionExpired {
                message: Some("invalid or expired token".into()),
            },
        ),
        // middleware/auth.rs:83 — logged in, wrong role. Not a session problem.
        (
            (403, Some("insufficient role".into())),
            ApiFailure::Http {
                status: 403,
                message: Some("insufficient role".into()),
            },
        ),
        (
            (500, None),
            ApiFailure::Http {
                status: 500,
                message: None,
            },
        ),
        // The client's own "never reached the backend" sentinel.
        ((0, None), ApiFailure::Transport),
    ];
    for (err, want) in cases {
        let got: ApiFailure = err.clone().into();
        assert_eq!(got, want, "classifying {err:?}");
        assert_eq!(
            got.is_session_expired(),
            err.0 == 401,
            "only the terminal 401 may say 'log in again' — {err:?}"
        );
    }
}

/// [`Fetched::view`] is **total**: it cannot be called without an answer for the failure case,
/// which is what stops a render site quietly falling through to its empty state.
#[test]
fn view_forces_the_failure_arm_to_exist_and_runs_it() {
    let dead: Fetched<Vec<u8>> = Err((401u16, None)).into();
    let rendered = dead.view(
        |rows| format!("{} rows", rows.len()),
        |f| {
            if f.is_session_expired() {
                "session expired — log in again".to_string()
            } else {
                "could not load".to_string()
            }
        },
    );
    assert_eq!(rendered, "session expired — log in again");

    let empty: Fetched<Vec<u8>> = Ok(Vec::new()).into();
    assert_eq!(
        empty.view(|rows| format!("{} rows", rows.len()), |_| "error".into()),
        "0 rows",
        "an empty success still renders as data — the empty state is correct HERE"
    );
}
