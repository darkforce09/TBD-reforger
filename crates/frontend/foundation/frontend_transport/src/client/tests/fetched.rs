//! A refusal and an empty success are different values, and the total reader proves it.

use super::Fetched;
use crate::error::Error;

/* ═════════════ a 401 is not an empty list ═════════════ */

/// The defect, stated as a value. An empty success and a dead session used to arrive at the render
/// site as the same thing — nothing, then an empty state. They are now different variants, and no
/// combinator on the type merges them.
#[test]
fn an_empty_result_and_a_401_are_different_values() {
    let empty: Fetched<Vec<u8>> = Ok(Vec::new()).into();
    let dead: Fetched<Vec<u8>> = Err(Error::from_status(401, None)).into();

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
    let cases: [((u16, Option<String>), Error); 4] = [
        (
            (401, Some("invalid or expired token".into())),
            Error::SessionExpired {
                message: Some("invalid or expired token".into()),
            },
        ),
        // middleware/auth.rs:83 — logged in, wrong role. Not a session problem.
        (
            (403, Some("insufficient role".into())),
            Error::Http {
                status: 403,
                message: Some("insufficient role".into()),
                details: None,
            },
        ),
        (
            (500, None),
            Error::Http {
                status: 500,
                message: None,
                details: None,
            },
        ),
        // The client's own "never reached the backend" sentinel.
        ((0, None), Error::Transport),
    ];
    for (err, want) in cases {
        let got = Error::from_status(err.0, err.1.clone());
        assert_eq!(got, want, "classifying {err:?}");
        assert_eq!(
            got.status(),
            err.0,
            "the status survives the classification — {err:?}"
        );
        assert_eq!(
            got.is_session_expired(),
            err.0 == 401,
            "only the terminal 401 may say 'log in again' — {err:?}"
        );
    }
}
