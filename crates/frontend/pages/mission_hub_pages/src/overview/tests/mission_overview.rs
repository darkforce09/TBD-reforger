//! The guards on the mission overview: the faction-key derivation, the save guard, the
//! request body and the briefing fallback.

use super::*;
use serde_json::Value;
use serde_json::json;

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

    assert!(
        draft_problem(&[row("BLUFOR", "  ", "1")])
            .unwrap()
            .contains("no name")
    );
    assert!(
        draft_problem(&[row("", "L85A3", "1")])
            .unwrap()
            .contains("blank faction key")
    );
    let padded = draft_problem(&[row("  USA  ", "L85A3", "1")]).unwrap();
    assert!(padded.contains("padded with whitespace"), "{padded}");
    // Names the reason it is refused rather than trimmed, so the operator fixes the ORBAT.
    assert!(padded.contains("Mission Creator"), "{padded}");
    assert!(
        draft_problem(&[row("BLUFOR", "L85A3", "many")])
            .unwrap()
            .contains("non-numeric quantity")
    );
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

/* ═════════════════════════ The status cell ═════════════════════════ */
