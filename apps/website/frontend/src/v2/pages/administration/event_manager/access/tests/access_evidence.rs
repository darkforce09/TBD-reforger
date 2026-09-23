//! What the access panel says: inheritance, participant evidence, change reports and refusals.

use super::change_report::{change_refusal_sentence, describe_registrations, is_revision_conflict};
use super::groups::account_names;
use super::member_search::member_search_path;
use super::participants_table::{
    authority_line, grants_line, guild_line, place_line, reservation_line,
};
use super::policy_inheritance::{slot_origin, squad_origin, PolicyOrigin};
use super::policy_lists::{policy_change_label, PolicyTarget};
use super::state::MissionSeats;
use super::waitlist_promotion::{promotion_refusal, promotion_report};
use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::api::dto::{
    DataEnvelope, EventAccessAdministration, OrbatSquad, ParticipantAccessExplanation,
};
use crate::v2::core::test_support::fixtures::golden;
use serde_json::json;

const ACCESS: &str = golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access.json");
const PARTICIPANTS: &str =
    golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access__participants.json");
const ORBAT: &str =
    golden!("GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json");
const MISSION: &str = "89b1b731-37a8-4926-901a-3c7ff7de5eb3";

fn access() -> EventAccessAdministration {
    serde_json::from_str(ACCESS).unwrap()
}

fn participants() -> Vec<ParticipantAccessExplanation> {
    serde_json::from_str(PARTICIPANTS).unwrap()
}

fn missions() -> Vec<MissionSeats> {
    let orbat: DataEnvelope<OrbatSquad> = serde_json::from_str(ORBAT).unwrap();
    vec![MissionSeats {
        event_mission_id: MISSION.into(),
        title: "Operation Byte Parity".into(),
        squads: orbat.data,
    }]
}

/* ───────────────────────── inheritance ───────────────────────── */

/// The captured overrides resolve the way the backend evaluates: the Recon squad has its own
/// policy, Bravo's fourth slot has its own, and everything else follows the operation's.
#[test]
fn captured_overrides_resolve_slot_then_squad_then_operation() {
    let access = access();
    assert!(squad_origin(&access, MISSION, "OPFOR", "Recon").is_own());
    assert!(matches!(
        squad_origin(&access, MISSION, "BLUFOR", "Alpha"),
        PolicyOrigin::Operation(_)
    ));
    assert!(slot_origin(
        &access,
        MISSION,
        "BLUFOR",
        "Bravo",
        "00000000-0000-4000-5000-000000000013"
    )
    .is_own());
    assert!(matches!(
        slot_origin(
            &access,
            MISSION,
            "OPFOR",
            "Recon",
            "00000000-0000-4000-5000-000000000014"
        ),
        PolicyOrigin::Squad(_)
    ));
    assert!(matches!(
        slot_origin(
            &access,
            MISSION,
            "BLUFOR",
            "Alpha",
            "00000000-0000-4000-5000-000000000004"
        ),
        PolicyOrigin::Operation(_)
    ));
}

/// An own policy with no grants is worded as admitting nobody, which inheritance never is.
#[test]
fn an_own_policy_without_grants_reads_as_admitting_nobody() {
    let mut access = access();
    access.squad_policies[0].policy.grants.clear();
    let origin = squad_origin(&access, MISSION, "OPFOR", "Recon");
    assert_eq!(
        origin.describe(&access),
        "Own policy with no grants: admits nobody"
    );
    let inherited = squad_origin(&access, MISSION, "BLUFOR", "Alpha").describe(&access);
    assert!(
        inherited.starts_with("Follows the operation's policy."),
        "{inherited}"
    );
    let recon_slot = slot_origin(&access, MISSION, "OPFOR", "Recon", "s").describe(&access);
    assert_eq!(recon_slot, "Follows its squad's policy. Admits nobody");
}

#[test]
fn policy_changes_are_labelled_by_target_and_kind() {
    let squad = PolicyTarget::Squad {
        event_mission_id: MISSION.into(),
        faction: "OPFOR".into(),
        squad: "Recon".into(),
    };
    assert_eq!(
        policy_change_label(&squad, false),
        "Policy of OPFOR / Recon saved"
    );
    assert_eq!(
        policy_change_label(&squad, true),
        "OPFOR / Recon inherits the operation's policy again"
    );
    assert_eq!(
        policy_change_label(&PolicyTarget::Operation, false),
        "Operation policy saved"
    );
}

/* ───────────────────────── participant evidence ───────────────────────── */

#[test]
fn grant_numbers_are_one_based() {
    assert_eq!(grants_line(&[]), "No grant admits it now");
    assert_eq!(grants_line(&[1]), "Admitted by grant 2");
    assert_eq!(grants_line(&[0, 2, 3]), "Admitted by grants 1, 3 and 4");
}

/// Only last-verified facts release a reservation, so current-only loss reads as standing.
#[test]
fn a_reservation_current_facts_no_longer_admit_still_stands() {
    assert!(authority_line(false, true).contains("stands"));
    assert!(authority_line(true, true).starts_with("Current and last-verified"));
    assert_ne!(authority_line(true, false), authority_line(false, false));
}

/// The captured evidence reads as seats by squad and role, pools with times, and guild checks.
#[test]
fn captured_evidence_reads_as_seats_places_and_guild_checks() {
    let list = participants();
    let missions = missions();
    let first = &list[0];
    assert_eq!(
        reservation_line(&first.registrations[0], &missions),
        "Operation Byte Parity — registered (BLUFOR / Command, 1. Platoon Leader); decided by the event policy"
    );
    assert_eq!(
        place_line(first),
        "Holds a member place since 2026-07-16 09:14 UTC"
    );
    assert_eq!(
        guild_line(&first.guilds[0]),
        "Guild 100000000000000001: unknown, never verified (not current)"
    );
    let waiting = list.iter().find(|p| p.allocation.is_none()).unwrap();
    assert_eq!(place_line(waiting), "Holds no place");
    assert!(reservation_line(&waiting.registrations[0], &missions).contains("waitlisted (no seat)"));
}

#[test]
fn account_names_come_from_rosters_and_participants() {
    let names = account_names(&access(), &participants());
    assert_eq!(
        names.get("000000000000000001").map(String::as_str),
        Some("Dev Operator")
    );
    assert_eq!(
        names.get("000000000000000006").map(String::as_str),
        Some("Kessler")
    );
}

/* ───────────────────────── change reports and refusals ───────────────────────── */

/// Released and promoted reservations are named from the participants read before the change,
/// and an unknown one keeps its id rather than vanishing from the count.
#[test]
fn changed_reservations_are_named_by_participant_and_mission() {
    let named = describe_registrations(
        &[
            "00000000-0000-4000-a100-000000000006".to_string(),
            "00000000-0000-4000-a100-00000000ffff".to_string(),
        ],
        &participants(),
        &missions(),
    );
    assert_eq!(
        named,
        vec![
            "Kessler — Operation Byte Parity".to_string(),
            "Registration 00000000-0000-4000-a100-00000000ffff".to_string(),
        ]
    );
}

/// A stale revision is told apart from every other conflict, and says the change was not applied.
#[test]
fn a_stale_revision_is_told_apart_from_other_conflicts() {
    let stale = ApiRefusal::from_error_body(
        409,
        Some(&json!({
            "error": "access settings changed since this form was loaded",
            "details": {"code": "ACCESS_REVISION_CONFLICT", "access_revision": 6}
        })),
    );
    assert!(is_revision_conflict(&stale));
    assert!(change_refusal_sentence(&stale).contains("was not applied"));
    let referenced = ApiRefusal::from_error_body(
        409,
        Some(
            &json!({"error": "an access policy still names this group; remove it from every policy first"}),
        ),
    );
    assert!(!is_revision_conflict(&referenced));
    assert_eq!(
        change_refusal_sentence(&referenced),
        "An access policy still names this group; remove it from every policy first"
    );
}

#[test]
fn a_promotion_reports_whom_it_seated_or_why_nobody() {
    let report = promotion_report(
        "Operation Byte Parity",
        vec!["Kessler — Operation Byte Parity".into()],
    );
    assert_eq!(
        report.change,
        "Seated from the waiting list of Operation Byte Parity"
    );
    assert_eq!(report.promoted.len(), 1);
    assert!(
        promotion_refusal("Operation Byte Parity", Some("EVENT_FULL"), "x".into()).contains("full")
    );
    assert_eq!(
        promotion_refusal("M", None, "Nobody waits".into()),
        "Nobody waits"
    );
}

/// The directory is searched only for a typed query, which is encoded into the path.
#[test]
fn the_member_search_asks_only_for_a_typed_query() {
    assert_eq!(member_search_path("   "), None);
    assert_eq!(
        member_search_path(" Rho des "),
        Some("/members?q=Rho%20des".to_string())
    );
}
