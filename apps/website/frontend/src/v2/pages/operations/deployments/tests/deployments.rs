//! The guards on the service record: the replay anchor sink, the leave date rules and request
//! body, the leave status badges, and the ban on fabricated personal telemetry.

use super::leave_of_absence::{create_leave_body, leave_status_variant, validate_loa_range};
use super::page::NO_TELEMETRY_RECORDED;
use super::service_record::replay_href;

// The same table both `is_http_url` implementations are pinned to — reused here so the CELL is
// checked against the adversarial corpus, not just the predicate underneath it. If a future
// edit reverts this cell to `!replay.is_empty()`, every `false` row stops returning `None` and
// the test below names the exact payload that would have rendered.
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../shared/is_http_url_cases.rs"
));

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

/// The specific regression, spelled out rather than left implicit in the table sweep: the
/// literal attack payload must produce no anchor, and the empty case must keep behaving exactly
/// as it did before the cell was guarded.
#[test]
fn the_t391_payload_renders_no_anchor_and_empty_still_means_no_link() {
    assert_eq!(replay_href("javascript:alert(1)"), None);
    assert_eq!(replay_href("JaVaScRiPt:alert(1)"), None);
    assert_eq!(replay_href("java\tscript:alert(1)"), None);
    assert_eq!(
        replay_href("data:text/html,<script>alert(1)</script>"),
        None
    );
    // Unchanged from before the guard: no replay uploaded yet renders the em dash.
    assert_eq!(replay_href(""), None);
    // ...and a real replay link still renders, which is the half that keeps the guard alive.
    assert_eq!(
        replay_href("https://aar.tbd/replays/abc.json"),
        Some("https://aar.tbd/replays/abc.json")
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

#[test]
fn leave_status_badge_variants() {
    assert_eq!(leave_status_variant("pending"), "warning");
    assert_eq!(leave_status_variant("approved"), "success");
    assert_eq!(leave_status_variant("denied"), "error");
    assert_eq!(leave_status_variant("bogus"), "neutral");
}

/// The personal kill/death, win-rate and favourite-loadout tiles were once hardcoded beside the
/// genuinely served deployment count. If any of these needles return, every operator sees the
/// same fabricated statistics again. Needles are `concat!`-split so this test does not match
/// itself. Do not restate the banned literals in comments above — paraphrase, or this goes red.
#[test]
fn no_fabricated_personal_telemetry_survives_in_this_module() {
    let src = crate::v2::core::test_support::pins::deployments_source();
    let banned = [
        concat!("MOCK_", "KD"),
        concat!("MOCK_", "WIN_RATE"),
        concat!("FAV_", "WEAPON_NAME"),
        concat!("FAV_", "ASSET_NAME"),
        concat!("FAV_", "WEAPON_IMG"),
        concat!("FAV_", "ASSET_IMG"),
        concat!("2.", "45"),
        concat!("68", "%"),
        concat!("M4A1 ", "Block II"),
        concat!("M1A2 ", "Abrams"),
        concat!("Telemetry", "Stat"),
        concat!("Fav", "Loadout"),
    ];
    for needle in banned {
        assert!(
            !src.contains(needle),
            "fabricated personal telemetry is back in deployments.rs: {needle:?}. \
             Until T-397, show the empty affordance — never invent numbers."
        );
    }
    assert!(
        src.contains(NO_TELEMETRY_RECORDED),
        "the honest empty affordance must stay on the page"
    );
}

#[test]
fn personal_telemetry_empty_copy_is_pinned() {
    assert_eq!(NO_TELEMETRY_RECORDED, "No telemetry recorded");
}
