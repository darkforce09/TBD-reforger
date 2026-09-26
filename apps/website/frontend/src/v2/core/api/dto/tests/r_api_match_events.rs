//! Captured-response round trips for a match's detailed events.

use super::*;

const EVENTS: &str = golden!("GET__matches__00000000-0000-4000-f000-000000000001__events.json");

/// Every key on the wire is read by a named field: the seven payloads are typed, so no payload
/// path is unclaimed.
#[test]
fn match_event_page() {
    assert_golden::<MatchEventPageDto>(EVENTS, &[]);
}

/// The captured page holds one event of each kind in sequence order, each decoded into the
/// variant its `kind` names, and it is the last page.
#[test]
fn each_kind_decodes_into_its_own_variant_in_sequence_order() {
    let page: MatchEventPageDto = serde_json::from_str(EVENTS).unwrap();
    assert_eq!(page.next_after_sequence, None, "one page holds all seven");
    let sequences: Vec<i64> = page.items.iter().map(|event| event.sequence).collect();
    assert_eq!(sequences, vec![1, 2, 3, 4, 5, 6, 7]);
    let kinds: Vec<&str> = page
        .items
        .iter()
        .map(|event| match &event.detail {
            MatchEventDetail::CombatKill(_) => "combat.kill",
            MatchEventDetail::CombatDeath(_) => "combat.death",
            MatchEventDetail::MedicalIncapacitated(_) => "medical.incapacitated",
            MatchEventDetail::MedicalRevived(_) => "medical.revived",
            MatchEventDetail::VehicleDestroyed(_) => "vehicle.destroyed",
            MatchEventDetail::VehicleEntered(_) => "vehicle.entered",
            MatchEventDetail::VehicleExited(_) => "vehicle.exited",
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "combat.kill",
            "medical.incapacitated",
            "medical.revived",
            "vehicle.entered",
            "vehicle.exited",
            "vehicle.destroyed",
            "combat.death",
        ]
    );
    let MatchEventDetail::CombatKill(kill) = &page.items[0].detail else {
        panic!("the first event is the kill");
    };
    assert_eq!(kill.distance_m.as_f64(), Some(38.5));
    assert!(kill.team_kill && kill.victim_is_player);
    let MatchEventDetail::CombatDeath(death) = &page.items[6].detail else {
        panic!("the last event is the death");
    };
    assert_eq!(death.cause, DeathCause::Ai);
}

/// A whole-metre distance stays an integer on the way back out, the optional kill fields stay
/// absent, and a page with more to read carries its cursor.
#[test]
fn a_whole_metre_kill_without_optionals_round_trips_exactly() {
    let wire = r#"{"items":[{"event_id":"k1","kind":"combat.kill","mission_time_ms":0,"occurred_at":"2026-06-20T19:00:00Z","payload":{"distance_m":42,"killer_arma_id":"a","team_kill":false,"victim_is_player":false},"sequence":1}],"next_after_sequence":1}"#;
    assert_golden::<MatchEventPageDto>(wire, &[]);
}

/// The four causes use the backend's spelling, `self` included.
#[test]
fn death_causes_use_the_wire_spelling() {
    for (cause, wire) in [
        (DeathCause::Ai, "ai"),
        (DeathCause::Environment, "environment"),
        (DeathCause::SelfInflicted, "self"),
        (DeathCause::Unknown, "unknown"),
    ] {
        assert_eq!(
            serde_json::to_value(cause).unwrap(),
            serde_json::json!(wire)
        );
    }
}

/// A kind the contract does not define is a decode error, not a silently dropped event.
#[test]
fn an_unknown_kind_is_rejected() {
    let wire = r#"{"event_id":"x","kind":"combat.unknown","mission_time_ms":0,"occurred_at":"t","payload":{},"sequence":1}"#;
    assert!(serde_json::from_str::<MatchEventDto>(wire).is_err());
}
