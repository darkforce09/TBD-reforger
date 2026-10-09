//! The guards on the service record: the replay anchor sink and the leave date rules and request
//! body.

use super::leave_of_absence::{create_leave_body, validate_loa_range};
use super::service_record::replay_href;
use http_url_guard::cases::IS_HTTP_URL_CASES;

// The guard's shared case table, reused here so the CELL is
// checked against the adversarial corpus, not just the predicate underneath it. If a future
// edit reverts this cell to `!replay.is_empty()`, every `false` row stops returning `None` and
// the test below names the exact payload that would have rendered.
#[test]
fn aar_cell_emits_an_href_only_for_http_urls() {
    let mut wrong = Vec::new();
    for (input, should_link) in IS_HTTP_URL_CASES {
        match (replay_href(input), should_link) {
            (Some(_), false) => wrong.push(format!("  RENDERED AN HREF FOR {input:?}")),
            (None, true) => wrong.push(format!("  refused a legitimate link {input:?}")),
            _ => {}
        }
    }
    assert!(
        wrong.is_empty(),
        "the AAR replay cell is wrong on {} of {} cases:\n{}",
        wrong.len(),
        IS_HTTP_URL_CASES.len(),
        wrong.join("\n")
    );
}

/// Mirrors `submit_leave` — empty, non-YMD, and inverted ranges must fail before the POST.
#[test]
fn loa_date_validation_matches_backend_rules() {
    assert_eq!(
        validate_loa_range("", "2026-08-05").unwrap_err(),
        "starts_on and ends_on are required"
    );
    assert_eq!(
        validate_loa_range("nope", "2026-08-05").unwrap_err(),
        "dates must be YYYY-MM-DD"
    );
    assert_eq!(
        validate_loa_range("2026-08-01T00:00:00Z", "2026-08-05").unwrap_err(),
        "dates must be YYYY-MM-DD"
    );
    assert_eq!(
        validate_loa_range("2026-08-05", "2026-08-01").unwrap_err(),
        "ends_on must be on or after starts_on"
    );
    assert!(validate_loa_range("2026-08-01", "2026-08-01").is_ok());
    assert!(validate_loa_range("2026-08-01", "2026-08-05").is_ok());
}

#[test]
fn create_leave_body_is_bare_ymd_json() {
    let v = create_leave_body("2026-08-01".into(), "2026-08-05".into(), "holiday".into());
    assert_eq!(
        v,
        serde_json::json!({
            "starts_on": "2026-08-01",
            "ends_on": "2026-08-05",
            "reason": "holiday",
        })
    );
}
