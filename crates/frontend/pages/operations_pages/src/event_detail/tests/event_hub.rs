//! The guards on the operation dossier: the reservation actions, the briefing empty rule and the
//! meta badges.

use super::slotting_selector::{can_register_reservation, can_withdraw_reservation};
use super::*;
use frontend_test_support::fixtures::golden;

#[test]
fn reservation_action_truth_table_distinguishes_active_allocations_from_retained_history() {
    for (state, register, withdraw) in [
        (None, true, false),
        (Some("registered"), false, true),
        (Some("waitlisted"), false, true),
        (Some("withdrawn"), true, false),
        (Some("legacy_unknown"), true, false),
        (Some("attended"), false, false),
        (Some("no_show"), false, false),
        (Some("future_reservation"), false, false),
        (Some(""), false, false),
        (Some(" registered"), false, false),
        (Some("Registered"), false, false),
    ] {
        let actual_register = can_register_reservation(state);
        let actual_withdraw = can_withdraw_reservation(state);
        assert_eq!(actual_register, register, "{state:?}");
        assert_eq!(actual_withdraw, withdraw, "{state:?}");
        assert!(
            !(actual_register && actual_withdraw),
            "reservation actions must be mutually exclusive"
        );
    }
}

#[test]
fn attendance_and_compatibility_state_do_not_control_reservation_actions() {
    for (reservation, register, withdraw) in [
        ("registered", false, true),
        ("waitlisted", false, true),
        ("withdrawn", true, false),
        ("legacy_unknown", true, false),
    ] {
        for attendance in [
            None,
            Some("attended"),
            Some("no_show"),
            Some("future_attendance"),
        ] {
            for compatibility in [
                None,
                Some("registered"),
                Some("attended"),
                Some("no_show"),
                Some("withdrawn"),
            ] {
                let mut dossier = golden_dossier();
                dossier.my_reservation_state = Some(reservation.to_owned());
                dossier.my_attendance_state = attendance.map(str::to_owned);
                dossier.my_state = compatibility.map(str::to_owned);
                assert_eq!(
                    can_register_reservation(dossier.my_reservation_state.as_deref()),
                    register
                );
                assert_eq!(
                    can_withdraw_reservation(dossier.my_reservation_state.as_deref()),
                    withdraw
                );
                assert_eq!(dossier.my_attendance_state.as_deref(), attendance);
                assert_eq!(dossier.my_state.as_deref(), compatibility);
            }
        }
    }
}

/// The event hub as the dev stack actually served it (the same capture the DTO golden
/// round-trips). **Its one mission carries no `briefing` key at all** — the backend omits the
/// field when the column is empty — so the single recorded real response is itself a live
/// instance of the defect, not a hypothetical one.
fn event_hub_golden() -> &'static str {
    golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json")
}

fn golden_dossier() -> EventMissionDossier {
    let hub: EventHub = serde_json::from_str(event_hub_golden()).expect("golden parses");
    hub.missions
        .into_iter()
        .next()
        .expect("golden has a mission")
}

/// The state that could not be reached before: an author clears the box, `PATCH` stores
/// `''`, the wire omits the key, and this arrives as `None`.
#[test]
fn a_cleared_briefing_renders_the_empty_state_and_never_the_invented_lore() {
    for cleared in [None, Some(""), Some("   \n\n  ")] {
        let out = briefing_text(cleared);
        assert_eq!(
            out, "No briefing provided.",
            "an authored-empty briefing must render the affordance, not prose ({cleared:?})"
        );
        // The specific claim: not one word of the removed lore, under any name.
        assert!(
            !out.contains("Hostile mechanized"),
            "the placeholder lore came back for {cleared:?}: {out}"
        );
        assert!(
            !out.contains("winter storm") && !out.contains("contested airspace"),
            "the placeholder lore came back for {cleared:?}: {out}"
        );
    }
}

/// The golden is the empty case — proof this is the live wire shape, not a synthetic one.
#[test]
fn the_recorded_wire_response_hits_the_empty_case() {
    let m = golden_dossier();
    assert!(
        m.briefing.is_none(),
        "fixture drifted: this test is only meaningful while the golden omits `briefing`"
    );
    assert_eq!(
        briefing_text(m.briefing.as_deref()),
        "No briefing provided."
    );
}

/// The operation-level briefing must use the same empty/trim rule on the hub hero that mission
/// dossiers already use. The golden omits the event key too.
#[test]
fn operation_level_briefing_uses_the_same_empty_rule() {
    let hub: EventHub = serde_json::from_str(event_hub_golden()).expect("golden parses");
    assert!(
        hub.briefing.is_none(),
        "fixture drifted: event-level briefing must be absent on the recorded wire"
    );
    assert_eq!(
        briefing_text(hub.briefing.as_deref()),
        "No briefing provided."
    );
    assert_eq!(
        briefing_text(Some("Hold the LZ until QRF.")),
        "Hold the LZ until QRF."
    );
    assert_eq!(
        briefing_text(Some("\n\n  ")),
        "No briefing provided.",
        "whitespace-only operation briefing is not authored content"
    );
}

/// The other half of the contract: a real briefing is still rendered verbatim, newlines and
/// all — the `<p>` is `whitespace-pre-line`, and the document core keeps paragraph breaks
/// intact specifically so they survive to a reader.
#[test]
fn an_authored_briefing_is_rendered_verbatim() {
    let authored = "Hold the ridge.\n\nSecond wave at H+20.";
    assert_eq!(briefing_text(Some(authored)), authored);
    // Leading/trailing space is not evidence of emptiness — only all-whitespace is.
    assert_eq!(briefing_text(Some(" Hold. ")), " Hold. ");
}

/// Every badge on the dossier header must come out of the dossier. This is the assertion the
/// fabricated `Maker`/`Duration` chips could not have survived: it pins the whole list, so
/// re-adding a hardcoded chip fails here rather than shipping.
#[test]
fn meta_badges_are_all_dossier_derived() {
    let m = golden_dossier();
    assert_eq!(meta_badges(&m), vec![("Terrain", "Everon".to_string())]);

    // Change the dossier, and every badge changes with it — nothing is pinned to a literal.
    let mut other = golden_dossier();
    other.terrain = "arland".into();
    assert_eq!(meta_badges(&other), vec![("Terrain", "Arland".to_string())]);
}
