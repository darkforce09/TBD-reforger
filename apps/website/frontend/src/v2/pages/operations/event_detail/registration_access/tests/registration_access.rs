//! The viewer's registration access, decided from the captured dossier and its variations.

use super::mission_standing::{release_reason_sentence, MissionStanding};
use super::place_outlook::{holds_place, place_outlook, PlaceOutlook};
use super::places_panel::{
    pool_availability_line, pool_badge_variant, pool_opening_line, quota_class_line,
    remaining_places_line,
};
use super::refusal_notices::{policy_owner, pool_name, RegistrationRefusal};
use super::seat_eligibility::{restriction_reason, seat_admits_viewer};
use super::waitlist_promotion::{promotion_refusal_sentence, promotion_summary};
use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::api::dto::{DataEnvelope, EventHub, OrbatSquad};
use crate::v2::core::test_support::fixtures::golden;
use serde_json::json;

const HUB: &str = golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json");
const ORBAT: &str =
    golden!("GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json");

/// The captured dossier, with `edit` applied to its wire form first.
fn hub_with(edit: impl FnOnce(&mut serde_json::Value)) -> EventHub {
    let mut wire: serde_json::Value = serde_json::from_str(HUB).unwrap();
    edit(&mut wire);
    serde_json::from_value(wire).unwrap()
}

/// The captured dossier as seen by a viewer with no signup on its one mission.
fn unsigned(edit: impl FnOnce(&mut serde_json::Value)) -> EventHub {
    hub_with(|wire| {
        let mission = &mut wire["missions"][0];
        for key in ["my_reservation_state", "my_state", "my_slot_id"] {
            mission.as_object_mut().unwrap().remove(key);
        }
        edit(wire);
    })
}

/// A stand-in for the viewer's local rendering, so sentences are tested without a browser clock.
fn local(iso: &str) -> String {
    format!("<local {iso}>")
}

/* ───────────────────────── the place outlook ───────────────────────── */

#[test]
fn a_registered_viewer_holds_the_operation_place() {
    let hub: EventHub = serde_json::from_str(HUB).unwrap();
    assert_eq!(
        hub.missions[0].my_reservation_state.as_deref(),
        Some("registered")
    );
    assert_eq!(place_outlook(&hub), PlaceOutlook::Held);
    for (state, holds) in [
        (Some("registered"), true),
        (Some("legacy_unknown"), true),
        (Some("waitlisted"), false),
        (Some("withdrawn"), false),
        (None, false),
    ] {
        assert_eq!(holds_place(state), holds, "{state:?}");
    }
}

/// The captured guest viewer's own pool has a place left, so a place is available.
#[test]
fn a_guest_draws_from_the_guest_pool_first() {
    assert_eq!(place_outlook(&unsigned(|_| {})), PlaceOutlook::Available);
}

/// With the guest pool full and the open pool closed, nothing is left for a guest — while a member
/// still draws from the uncapped member pool.
#[test]
fn exhausted_pools_leave_no_place_but_only_for_the_viewer_who_draws_from_them() {
    let full_guest = |wire: &mut serde_json::Value| {
        let guest = &mut wire["reservation_quotas"][1];
        guest["remaining"] = json!(0);
        guest["allocated"] = json!(2);
        guest["open"] = json!(false);
        guest["closed_reason"] = json!("full");
    };
    assert_eq!(
        place_outlook(&unsigned(full_guest)),
        PlaceOutlook::Exhausted
    );
    let member = unsigned(|wire| {
        full_guest(wire);
        wire["viewer_access"]["quota_class"] = json!("member");
    });
    assert_eq!(place_outlook(&member), PlaceOutlook::Available);
}

/// The operation-wide limit wins over every pool.
#[test]
fn a_full_operation_has_no_place_whatever_the_pools_say() {
    let hub = unsigned(|wire| wire["remaining_event_places"] = json!(0));
    assert_eq!(place_outlook(&hub), PlaceOutlook::Exhausted);
}

/// When no pool is open yet, the earliest opening of a pool with places is reported — compared by
/// instant, not by text, and with the viewer's own pool winning a tie.
#[test]
fn the_earliest_opening_is_reported_when_nothing_is_open_yet() {
    let not_yet = |pool: &mut serde_json::Value, at: &str| {
        pool["open"] = json!(false);
        pool["closed_reason"] = json!("not_yet_open");
        pool["opens_at"] = json!(at);
        pool["seat_limit"] = json!(5);
        pool["remaining"] = json!(5);
    };
    let hub = unsigned(|wire| {
        not_yet(&mut wire["reservation_quotas"][1], "2026-08-01T12:00:00.5Z");
        not_yet(&mut wire["reservation_quotas"][2], "2026-08-01T12:00:00Z");
    });
    assert_eq!(
        place_outlook(&hub),
        PlaceOutlook::OpensLater {
            quota_kind: "open".into(),
            opens_at: "2026-08-01T12:00:00Z".into()
        }
    );
    let tie = unsigned(|wire| {
        not_yet(&mut wire["reservation_quotas"][1], "2026-08-01T12:00:00Z");
        not_yet(&mut wire["reservation_quotas"][2], "2026-08-01T12:00:00Z");
    });
    assert!(matches!(
        place_outlook(&tie),
        PlaceOutlook::OpensLater { quota_kind, .. } if quota_kind == "guest"
    ));
}

/// A closed reason this build does not know reads as closed.
#[test]
fn an_unknown_pool_state_reads_as_closed() {
    let hub = unsigned(|wire| {
        let guest = &mut wire["reservation_quotas"][1];
        guest["open"] = json!(false);
        guest["closed_reason"] = json!("suspended");
    });
    assert_eq!(place_outlook(&hub), PlaceOutlook::Exhausted);
}

/* ───────────────────────── the mission standing ───────────────────────── */

#[test]
fn the_standing_is_read_from_the_dossier() {
    let hub: EventHub = serde_json::from_str(HUB).unwrap();
    let standing = MissionStanding::of(&hub, &hub.missions[0]);
    assert_eq!(standing.reservation_state.as_deref(), Some("registered"));
    assert!(standing.eligible && !standing.verification_pending && !standing.mission_full);
    assert_eq!(standing.outlook, PlaceOutlook::Held);
    assert!(
        !standing.seat_claim_offered(),
        "a registered viewer claims no second seat"
    );
    assert!(!standing.waiting_list_offered(true));
    assert_eq!(
        standing.signup_line().as_deref(),
        Some("You are registered for this mission.")
    );
}

/// A reservation recorded before states were tracked holds its place like a registration: it is
/// stated as one, and without a seat it may still take a free seat.
#[test]
fn a_legacy_reservation_holds_its_place_like_a_registration() {
    let seated =
        hub_with(|wire| wire["missions"][0]["my_reservation_state"] = json!("legacy_unknown"));
    let standing = MissionStanding::of(&seated, &seated.missions[0]);
    assert_eq!(standing.outlook, PlaceOutlook::Held);
    assert_eq!(
        standing.signup_line().as_deref(),
        Some("You are registered for this mission.")
    );
    let seatless = hub_with(|wire| {
        wire["missions"][0]["my_reservation_state"] = json!("legacy_unknown");
        wire["missions"][0]["my_slot_id"] = serde_json::Value::Null;
    });
    let standing = MissionStanding::of(&seatless, &seatless.missions[0]);
    assert!(standing
        .signup_line()
        .unwrap()
        .starts_with("You hold a place"));
    assert!(standing.seat_claim_offered());
}

/// A full mission offers the waiting list instead of a seat, and only to a viewer some seat of it
/// admits.
#[test]
fn a_full_mission_offers_the_waiting_list_to_an_eligible_viewer_only() {
    let hub = unsigned(|wire| wire["missions"][0]["filled"] = json!(16));
    let standing = MissionStanding::of(&hub, &hub.missions[0]);
    assert!(standing.mission_full);
    assert!(!standing.seat_claim_offered());
    assert!(standing.waiting_list_offered(false));
    let barred = unsigned(|wire| {
        wire["missions"][0]["filled"] = json!(16);
        wire["missions"][0]["viewer_eligible"] = json!(false);
    });
    assert!(!MissionStanding::of(&barred, &barred.missions[0]).waiting_list_offered(true));
}

/// An open mission with a place to be had offers the seat claim; the waiting list appears only
/// once a refusal says there is no place after all.
#[test]
fn an_open_mission_offers_a_seat_and_the_waiting_list_only_after_a_full_refusal() {
    let hub = unsigned(|_| {});
    let standing = MissionStanding::of(&hub, &hub.missions[0]);
    assert!(standing.seat_claim_offered());
    assert!(!standing.waiting_list_offered(false));
    assert!(standing.waiting_list_offered(true));
}

/// A waiting viewer is offered neither action, and is told their position.
#[test]
fn a_waiting_viewer_is_told_their_position() {
    let hub = unsigned(|wire| {
        wire["missions"][0]["my_reservation_state"] = json!("waitlisted");
        wire["missions"][0]["my_waiting_position"] = json!(2);
    });
    let standing = MissionStanding::of(&hub, &hub.missions[0]);
    assert_eq!(standing.waiting_position, Some(2));
    assert!(!standing.seat_claim_offered() && !standing.waiting_list_offered(true));
    assert_eq!(
        standing.signup_line().as_deref(),
        Some("You are on the waiting list, position 2.")
    );
}

/// A viewer registered without a seat already holds the place, and may take a free seat — which
/// a seated viewer is not offered.
#[test]
fn a_registered_viewer_without_a_seat_may_take_one() {
    let hub = hub_with(|wire| {
        wire["missions"][0]
            .as_object_mut()
            .unwrap()
            .remove("my_slot_id");
    });
    let standing = MissionStanding::of(&hub, &hub.missions[0]);
    assert!(!standing.holds_seat);
    assert_eq!(standing.outlook, PlaceOutlook::Held);
    assert!(standing.seat_claim_offered());
    assert!(!standing.waiting_list_offered(true));
    assert_eq!(
        standing.signup_line().as_deref(),
        Some("You hold a place on this mission without a seat; pick a free seat to take one.")
    );
}

/// A released signup keeps its reason and time, and may sign up again.
#[test]
fn a_released_signup_keeps_its_reason_and_may_sign_up_again() {
    let hub = unsigned(|wire| {
        let mission = &mut wire["missions"][0];
        mission["my_reservation_state"] = json!("withdrawn");
        mission["my_release_reason"] = json!("eligibility_lost");
        mission["my_withdrawn_at"] = json!("2026-07-20T18:30:00Z");
    });
    let standing = MissionStanding::of(&hub, &hub.missions[0]);
    assert_eq!(standing.release_reason.as_deref(), Some("eligibility_lost"));
    assert_eq!(
        standing.released_at.as_deref(),
        Some("2026-07-20T18:30:00Z")
    );
    assert!(standing.seat_claim_offered());
    assert_eq!(standing.signup_line(), None);
}

/// Every documented release reason has its own sentence, and an unknown one still reads.
#[test]
fn every_release_reason_has_its_own_sentence() {
    let reasons = [
        "participant_withdrew",
        "mission_removed",
        "event_cancelled",
        "event_deleted",
        "eligibility_lost",
        "access_policy_changed",
        "account_unavailable",
        "seat_cleared",
    ];
    let sentences: std::collections::BTreeSet<String> =
        reasons.iter().map(|r| release_reason_sentence(r)).collect();
    assert_eq!(
        sentences.len(),
        reasons.len(),
        "each reason reads differently"
    );
    assert!(sentences.iter().all(|s| !s.contains('_')));
    assert_eq!(
        release_reason_sentence("seat_swapped"),
        "Released: seat swapped."
    );
}

#[test]
fn an_unlisted_mission_withholds_no_offer() {
    let standing = MissionStanding::unlisted();
    assert!(standing.seat_claim_offered());
    assert!(!standing.waiting_list_offered(false));
}

/* ───────────────────────── refusals ───────────────────────── */

fn refused(code: &str, extra: serde_json::Value) -> ApiRefusal {
    let mut details = json!({"code": code});
    for (key, value) in extra.as_object().unwrap() {
        details[key] = value.clone();
    }
    ApiRefusal::from_error_body(
        409,
        Some(&json!({"error": "backend sentence", "details": details})),
    )
}

/// Every documented refusal reason is recognised, and none falls through to the backend sentence.
#[test]
fn every_documented_refusal_reason_is_recognised() {
    for code in [
        "ACCESS_POLICY",
        "MEMBERSHIP_VERIFICATION_REQUIRED",
        "QUOTA_NOT_OPEN",
        "EVENT_FULL",
        "MISSION_FULL",
        "SEAT_NEEDED_BY_HOLDER",
        "SEAT_TAKEN",
        "SQUAD_HELD",
        "NO_SEATS",
        "REGISTRATION_CLOSED",
        "ACCOUNT_UNAVAILABLE",
        "DEPLOYMENT_REQUIREMENTS",
    ] {
        let refusal = RegistrationRefusal::from_refusal(&refused(code, json!({})), "fallback");
        assert!(
            !matches!(refusal, RegistrationRefusal::Other(_)),
            "{code} must be recognised"
        );
        let sentence = refusal.sentence(local);
        assert!(
            !sentence.is_empty() && sentence != "Backend sentence",
            "{code}: {sentence}"
        );
    }
}

/// A pool that has not opened names its opening in the viewer's zone with the UTC time beside it.
#[test]
fn a_pool_not_yet_open_names_its_opening_locally_and_in_utc() {
    let refusal = RegistrationRefusal::from_refusal(
        &refused(
            "QUOTA_NOT_OPEN",
            json!({"quota_kind": "guest", "opens_at": "2026-08-01T12:00:00Z"}),
        ),
        "fallback",
    );
    assert_eq!(
        refusal.sentence(local),
        "Guest places open <local 2026-08-01T12:00:00Z> (2026-08-01 12:00 UTC)."
    );
}

/// An access-policy refusal names the policy that refused, by its source.
#[test]
fn an_access_policy_refusal_names_the_refusing_policy() {
    for (source, owner) in [
        ("slot", "This seat's access policy"),
        ("squad", "This squad's access policy"),
        ("event", "The operation's access policy"),
    ] {
        let refusal = RegistrationRefusal::from_refusal(
            &refused("ACCESS_POLICY", json!({"policy_source": source})),
            "fallback",
        );
        assert_eq!(
            refusal.sentence(local),
            format!("{owner} does not admit you to this seat.")
        );
    }
    assert_eq!(policy_owner(None), "An access policy");
}

/// A membership verification in progress is said as such, not as a denial.
#[test]
fn a_pending_verification_is_explained_not_denied() {
    let refusal = RegistrationRefusal::from_refusal(
        &refused(
            "MEMBERSHIP_VERIFICATION_REQUIRED",
            json!({"policy_source": "event"}),
        ),
        "fallback",
    );
    assert!(refusal.sentence(local).contains("being verified"));
    assert!(!refusal.suggests_waiting_list());
}

/// Only the full-operation, full-mission and last-seat refusals point at the waiting list.
#[test]
fn only_capacity_refusals_point_at_the_waiting_list() {
    for (code, suggests) in [
        ("EVENT_FULL", true),
        ("MISSION_FULL", true),
        ("SEAT_NEEDED_BY_HOLDER", true),
        ("SEAT_TAKEN", false),
        ("ACCESS_POLICY", false),
        ("QUOTA_NOT_OPEN", false),
    ] {
        let refusal = RegistrationRefusal::from_refusal(&refused(code, json!({})), "fallback");
        assert_eq!(refusal.suggests_waiting_list(), suggests, "{code}");
    }
}

/// A refusal without a known reason shows the backend's sentence, or the fallback without one.
#[test]
fn an_unknown_refusal_shows_the_backend_sentence() {
    let unknown = RegistrationRefusal::from_refusal(&refused("NEW_REASON", json!({})), "fallback");
    assert_eq!(unknown.sentence(local), "Backend sentence");
    let bare = RegistrationRefusal::from_refusal(&ApiRefusal::unreadable(), "Could not register");
    assert_eq!(bare.sentence(local), "Could not register");
    assert_eq!(pool_name(Some("partner_guest")), "Partner guest places");
}

/* ───────────────────────── seats ───────────────────────── */

/// Only an eligible seat admits the viewer; a restricted one names the policy that restricts it.
#[test]
fn restricted_seats_are_closed_and_say_why() {
    let orbat: DataEnvelope<OrbatSquad> = serde_json::from_str(ORBAT).unwrap();
    let seats: Vec<_> = orbat.data.iter().flat_map(|squad| &squad.slots).collect();
    let restricted: Vec<_> = seats.iter().filter(|s| !seat_admits_viewer(s)).collect();
    assert!(!restricted.is_empty() && restricted.len() < seats.len());
    assert!(restricted.iter().all(|s| s.viewer_access == "restricted"));
    let mut odd = (*seats[0]).clone();
    odd.viewer_access = "pending".into();
    assert!(
        !seat_admits_viewer(&odd),
        "an unknown standing reads as restricted"
    );
    assert_eq!(
        restriction_reason("slot"),
        "Restricted by this seat's access policy"
    );
    assert_eq!(
        restriction_reason("squad"),
        "Restricted by this squad's access policy"
    );
    assert_eq!(
        restriction_reason("event"),
        "Restricted by the operation's access policy"
    );
    assert_eq!(restriction_reason("unit"), "Restricted by an access policy");
}

/* ───────────────────────── the Places panel ───────────────────────── */

/// The captured pools read as an uncapped open pool, a capped one with a place left, and a closed
/// one with none; the two open pools state when they opened, locally and in UTC, and the pool
/// without places states no opening.
#[test]
fn the_captured_pools_read_as_their_state() {
    let hub: EventHub = serde_json::from_str(HUB).unwrap();
    let lines: Vec<String> = hub
        .reservation_quotas
        .iter()
        .map(pool_availability_line)
        .collect();
    assert_eq!(
        lines,
        [
            "Open now · no pool limit, 4 taken",
            "Open now · 1 of 2 places left",
            "Closed · this pool has no places",
        ]
    );
    let openings: Vec<Option<String>> = hub
        .reservation_quotas
        .iter()
        .map(|pool| pool_opening_line(pool, local))
        .collect();
    let opened = "Opened <local 2026-07-15T14:05:44.629713Z> (2026-07-15 14:05 UTC)".to_string();
    assert_eq!(openings, [Some(opened.clone()), Some(opened), None]);
    let variants: Vec<&str> = hub
        .reservation_quotas
        .iter()
        .map(pool_badge_variant)
        .collect();
    assert_eq!(variants, ["success", "success", "neutral"]);
}

/// A pool that has not opened states when it opens, locally and in UTC, and a full one its count.
#[test]
fn a_pool_line_states_its_opening_or_its_count() {
    let hub = hub_with(|wire| {
        let guest = &mut wire["reservation_quotas"][1];
        guest["open"] = json!(false);
        guest["closed_reason"] = json!("not_yet_open");
        guest["opens_at"] = json!("2026-08-01T12:00:00Z");
        let open = &mut wire["reservation_quotas"][2];
        open["seat_limit"] = json!(3);
        open["allocated"] = json!(3);
        open["remaining"] = json!(0);
        open["closed_reason"] = json!("full");
    });
    assert_eq!(
        pool_availability_line(&hub.reservation_quotas[1]),
        "Not open yet · 1 of 2 places left"
    );
    assert_eq!(
        pool_opening_line(&hub.reservation_quotas[1], local).as_deref(),
        Some("Opens <local 2026-08-01T12:00:00Z> (2026-08-01 12:00 UTC)")
    );
    assert_eq!(
        pool_availability_line(&hub.reservation_quotas[2]),
        "Full · 3 of 3 taken"
    );
    assert!(pool_opening_line(&hub.reservation_quotas[2], local)
        .is_some_and(|line| line.starts_with("Opened ")));
}

#[test]
fn the_operation_remainder_and_the_viewer_pool_read_as_sentences() {
    assert_eq!(remaining_places_line(None), "No operation-wide limit");
    assert_eq!(remaining_places_line(Some(0)), "The operation is full");
    assert_eq!(
        remaining_places_line(Some(1)),
        "1 place left in the operation"
    );
    assert_eq!(
        remaining_places_line(Some(7)),
        "7 places left in the operation"
    );
    assert_eq!(
        quota_class_line("guest"),
        "Your place comes from guest places first, then from open places once they open."
    );
}

/* ───────────────────────── waiting-list promotion ───────────────────────── */

#[test]
fn a_promotion_reports_how_many_were_seated_or_why_none_were() {
    assert_eq!(
        promotion_summary(1),
        "Seated 1 participant from the waiting list."
    );
    assert_eq!(
        promotion_summary(3),
        "Seated 3 participants from the waiting list."
    );
    assert!(promotion_refusal_sentence(Some("EVENT_FULL"), "x".into()).contains("full"));
    assert_eq!(
        promotion_refusal_sentence(None, "No participant is waiting for this mission".into()),
        "No participant is waiting for this mission"
    );
}
