//! The guards on the mission overview: the faction-key derivation, the save guard, the
//! request body, the briefing fallback and the status cell.

use super::*;
use crate::v2::core::test_support::fixtures::golden;
use serde_json::json;
use serde_json::Value;

fn row(faction: &str, item: &str, qty: &str) -> DraftRow {
    DraftRow {
        faction: faction.into(),
        item_name: item.into(),
        category: String::new(),
        quantity: qty.into(),
    }
}

/// The live path: saving a version omits the top-level order of battle, so the keys come
/// from the editor graph's factions instead, copied verbatim into each slot's faction.
#[test]
fn derives_editor_faction_keys_verbatim() {
    let p = json!({
        "editor": {
            "factions": [
                {"key": "BLUFOR", "squadIds": ["sq1"]},
                {"key": "  USA  ", "squadIds": ["sq2"]},
            ],
            "squads": [
                {"id": "sq1", "slotIds": ["s1", "s2"]},
                {"id": "sq2", "slotIds": ["s3"]},
            ],
            "slots": [{"id": "s1"}, {"id": "s2"}, {"id": "s3"}],
        }
    });
    // Padding is carried, not trimmed: the other side of the join carries it too.
    assert_eq!(orbat_faction_keys(&p), vec!["BLUFOR", "  USA  "]);
}

/// `materialize_slots` inserts one row per slot, so a faction whose squads resolve to no slot
/// produces no `orbat_slots` row and appears in no Event Hub faction list. Offering it would be
/// offering a key that joins to nothing.
#[test]
fn omits_factions_that_materialise_nothing() {
    let p = json!({
        "editor": {
            "factions": [
                {"key": "EMPTY_SQUAD", "squadIds": ["sq1"]},
                {"key": "NO_SQUADS", "squadIds": []},
                {"key": "DANGLING_SQUAD", "squadIds": ["nope"]},
                {"key": "DANGLING_SLOTS", "squadIds": ["sq2"]},
                {"key": "REAL", "squadIds": ["sq3"]},
            ],
            "squads": [
                {"id": "sq1", "slotIds": []},
                {"id": "sq2", "slotIds": ["ghost"]},
                {"id": "sq3", "slotIds": ["s1"]},
            ],
            "slots": [{"id": "s1"}],
        }
    });
    assert_eq!(orbat_faction_keys(&p), vec!["REAL"]);
}

/// An explicit non-empty `orbat` array wins the precedence test, exactly as
/// `parse_orbat_template`'s early return does — the editor graph is then never consulted.
#[test]
fn explicit_orbat_array_wins_and_dedupes() {
    let p = json!({
        "orbat": [
            {"faction": "USA", "slots": [{"role": "SL"}]},
            {"faction": "USA", "slots": [{"role": "RTO"}]},
            {"faction": "RU", "slots": []},
        ],
        "editor": {
            "factions": [{"key": "NEVER_REACHED", "squadIds": ["sq1"]}],
            "squads": [{"id": "sq1", "slotIds": ["s1"]}],
            "slots": [{"id": "s1"}],
        }
    });
    // `RU` has no slot, so it materialises nothing; `USA` appears once.
    assert_eq!(orbat_faction_keys(&p), vec!["USA"]);
}

/// A malformed `orbat` fails to decode server-side and `unwrap_or_default()` falls through to
/// the editor graph. Taking the other branch here would offer keys from a source the server
/// never reads.
#[test]
fn malformed_orbat_falls_through_like_serde() {
    let editor = json!({
        "factions": [{"key": "BLUFOR", "squadIds": ["sq1"]}],
        "squads": [{"id": "sq1", "slotIds": ["s1"]}],
        "slots": [{"id": "s1"}],
    });
    // An object (every compiled golden mission's shape), an array of non-objects, and a
    // wrongly-typed field all fail `Top`'s decode.
    for bad in [
        json!({"blufor": {}}),
        json!(["BLUFOR"]),
        json!([{"faction": 7}]),
        json!([{"slots": "many"}]),
    ] {
        let p = json!({ "orbat": bad, "editor": editor });
        assert_eq!(
            orbat_faction_keys(&p),
            vec!["BLUFOR"],
            "should have fallen through for {bad}"
        );
    }
    // An EMPTY array also loses the precedence test, matching `if !top.orbat.is_empty()`.
    let p = json!({ "orbat": [], "editor": editor });
    assert_eq!(orbat_faction_keys(&p), vec!["BLUFOR"]);
}

#[test]
fn no_payload_shape_yields_no_keys() {
    for p in [
        json!({}),
        json!({"editor": {}}),
        json!({"editor": {"factions": []}}),
        json!({"editor": {"factions": [{"key": "X"}]}}), // no squadIds at all
    ] {
        assert!(orbat_faction_keys(&p).is_empty(), "for {p}");
    }
}

/// The server's predicate, not an approximation of it (`handlers/missions.rs:769`, `:777`).
#[test]
fn key_storable_matches_the_server_guard() {
    assert!(key_storable("USA"));
    assert!(key_storable("BLU FOR")); // interior space is fine; only the ends are refused
    assert!(!key_storable(""));
    assert!(!key_storable("   "));
    assert!(!key_storable("  USA  "));
    assert!(!key_storable("USA "));
    assert!(!key_storable("\tUSA"));
}

#[test]
fn parse_qty_blank_is_unlimited_and_junk_is_refused() {
    assert_eq!(parse_qty(""), Some(None));
    assert_eq!(parse_qty("   "), Some(None));
    assert_eq!(parse_qty("12"), Some(Some(12)));
    assert_eq!(parse_qty(" 12 "), Some(Some(12)));
    assert_eq!(parse_qty("-3"), Some(Some(-3))); // the server accepts it; we do not invent a rule
    assert_eq!(parse_qty("1.5"), None);
    assert_eq!(parse_qty("lots"), None);
}

#[test]
fn draft_problem_flags_exactly_what_the_endpoint_refuses() {
    assert!(draft_problem(&[]).is_none());
    assert!(draft_problem(&[row("BLUFOR", "L85A3", "12")]).is_none());
    assert!(draft_problem(&[row("BLUFOR", "L85A3", "")]).is_none());

    assert!(draft_problem(&[row("BLUFOR", "  ", "1")])
        .unwrap()
        .contains("no name"));
    assert!(draft_problem(&[row("", "L85A3", "1")])
        .unwrap()
        .contains("blank faction key"));
    let padded = draft_problem(&[row("  USA  ", "L85A3", "1")]).unwrap();
    assert!(padded.contains("padded with whitespace"), "{padded}");
    // Names the reason it is refused rather than trimmed, so the operator fixes the ORBAT.
    assert!(padded.contains("Mission Creator"), "{padded}");
    assert!(draft_problem(&[row("BLUFOR", "L85A3", "many")])
        .unwrap()
        .contains("non-numeric quantity"));
}

/// `items` is always present — an absent one is a decode failure by design, because the
/// first statement is an unconditional DELETE.
#[test]
fn armory_body_always_states_items() {
    assert_eq!(armory_body(&[]), json!({ "items": [] }));
}

#[test]
fn armory_body_sends_the_key_verbatim_and_orders_rows() {
    let rows = vec![row("BLUFOR", " L85A3 ", "12"), row("OPFOR", "AK-74", "")];
    let body = armory_body(&rows);
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    // Verbatim on both, for opposite reasons: the server trims the label and refuses a padded
    // key, and both of those decisions are its to make.
    assert_eq!(items[0]["faction"], json!("BLUFOR"));
    assert_eq!(items[0]["item_name"], json!(" L85A3 "));
    assert_eq!(items[0]["quantity"], json!(12));
    assert_eq!(items[0]["sort_order"], json!(0));
    // Blank quantity is `null`, which the column reads as unlimited — not 0.
    assert_eq!(items[1]["quantity"], Value::Null);
    assert_eq!(items[1]["sort_order"], json!(1));
}

/// Whitespace-only briefings must not render as blank "authored" prose under the
/// "Tactical Briefing" heading. The same trim rule the rest of the platform uses.
#[test]
fn tactical_briefing_trims_whitespace_only_to_empty_affordance() {
    for cleared in [None, Some(""), Some("   \n\n  "), Some("\t")] {
        assert_eq!(
            tactical_briefing_text(cleared),
            "No briefing provided.",
            "whitespace-only briefing must take the empty affordance ({cleared:?})"
        );
    }
    let authored = "Hold the ridge.\n\nSecond wave at H+20.";
    assert_eq!(tactical_briefing_text(Some(authored)), authored);
    // Leading/trailing space alone is not emptiness — only all-whitespace is.
    assert_eq!(tactical_briefing_text(Some(" Hold. ")), " Hold. ");
}

/// Guard against reverting to the emptiness check without a trim.
///
/// The filter-only ratchet once stayed green while a match arm without a trim made the
/// behavioural test go red. Ban both shapes, and pin the trim-aware arm so a rewrite
/// cannot drop the trim unnoticed.
#[test]
fn dossier_body_uses_trim_aware_briefing_helper() {
    let src = crate::v2::core::test_support::pins::mission_overview_source();
    assert!(
        src.contains("tactical_briefing_text(m.briefing.as_deref())"),
        "dossier_body must route briefing through tactical_briefing_text"
    );
    // concat! so this test body does not match itself.
    let old_filter = concat!(".filter(|b| !b.", "is_empty())");
    assert!(
        !src.contains(old_filter),
        "the pre-T-407 is_empty-only filter must not return — whitespace-only \
             briefings would blank the Tactical Briefing section again"
    );
    let old_arm = concat!("Some(b) if !b.", "is_empty()");
    assert!(
        !src.contains(old_arm),
        "match-arm !b.is_empty() without trim must not return on briefing paths — \
             whitespace-only briefings would blank the Tactical Briefing section again"
    );
    let trim_arm = concat!("Some(b) if !b.trim().", "is_empty()");
    assert!(
        src.contains(trim_arm),
        "tactical_briefing_text must keep the trim-aware match arm"
    );
}

/* ═════════════════════════ The status cell ═════════════════════════ */

/// The real captured wire body of a mission that was driven through submit → reject
/// (`dto.rs mission_detail_rejected_carries_the_review_stamp` round-trips the same file).
/// Deserialised rather than hand-built so the test is driven by a status the backend really
/// sends, not by a string this file invented.
const REJECTED_GOLDEN: &str = golden!("GET__missions__82b937fc-c88e-4bb9-abb3-0bef67379398.json");

/// **Red without the fix.** The dossier grid used to render the status verbatim, so this is
/// the raw enum token on the unfixed tree — lowercase, in a headline cell directly under a
/// "Returned by review" callout saying the opposite.
///
/// Behavioural, not a grep: [`detail_rows`] is what `dossier_body` builds the `<dl>` from, so
/// putting `m.status.clone()` back into the grid means putting it back here.
#[test]
fn the_dossier_status_cell_is_labelled_not_the_raw_enum() {
    let m: MissionDetail = serde_json::from_str(REJECTED_GOLDEN).expect("rejected golden");
    assert_eq!(m.status, "rejected", "fixture must be the returned mission");
    let rows = detail_rows(&m);
    let status = rows
        .iter()
        .find(|(l, _)| *l == "Status")
        .expect("the grid must still carry a Status cell");
    assert_eq!(
        status.1, "Returned",
        "the STATUS cell must render the same label the card badge does"
    );
    assert_ne!(
        status.1, m.status,
        "rendering the DB enum verbatim is the T-395 defect"
    );
}

/// Every value the enum can hold gets a label, and no label is the raw token. `other` is
/// covered too: an enum value added tomorrow must not reach the screen as `some_new_state`.
#[test]
fn no_mission_status_reaches_the_screen_as_a_database_token() {
    for status in [
        "draft",
        "pending_approval",
        "live",
        "rejected",
        "archived",
        "some_future_state",
    ] {
        let label = mission_status_label(status);
        assert!(!label.is_empty(), "{status}: empty label");
        assert!(
            !label.contains('_'),
            "{status}: `{label}` still carries the snake_case join of a DB token"
        );
        assert_ne!(label, status, "{status}: label is the raw enum value");
    }
}

/// **The drift lock.** Two copies of one mapping is how the grid and the badge came to
/// disagree, so pin the call itself, on the scrubbed production half of the library source,
/// which folds comments, dead configuration items and false blocks away and fails closed on
/// anything it cannot read.
///
/// `live_code` blanks string literals as well, so a `"mission_status_label"` mention inside a
/// doc string or a copy literal cannot green this.
#[test]
fn the_card_badge_and_the_dossier_grid_share_one_label_mapper() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let prod = live_code(&crate::v2::core::test_support::pins::mission_library_source());
    let badge = only_body(&prod, "fn visibility_badge(status: &str)");
    assert!(
        badge.contains("mission_status_label(status)"),
        "visibility_badge must take its label from mission_overview::mission_status_label, \
             not a second local match — that duplication is T-395. Body was: {badge}"
    );
    assert!(
        !badge.contains("=> ("),
        "visibility_badge must not rebuild the (label, variant) tuple match — the label half \
             moved to mission_status_label. Body was: {badge}"
    );
    // And the grid: the view is built from detail_rows, so a hand-rolled Status cell beside it
    // would be a second source the behavioural test above cannot see.
    let own = live_code(&crate::v2::core::test_support::pins::mission_overview_source());
    let body = only_body(&own, "pub fn dossier_body(m: &MissionDetail)");
    assert!(
        body.contains("detail_rows(m)"),
        "dossier_body must build the detail grid from detail_rows"
    );
    assert!(
        !body.contains("m.status"),
        "dossier_body must not touch m.status directly — that is the raw-enum path T-395 removed"
    );
}
