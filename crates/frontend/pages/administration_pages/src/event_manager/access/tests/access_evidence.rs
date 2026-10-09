//! The access panel's reading of the captured access: policy inheritance, account names, the
//! revision conflict and the member search path.

use super::change_report::is_revision_conflict;
use super::groups::account_names;
use super::member_search::member_search_path;
use super::policy_inheritance::{PolicyOrigin, slot_origin, squad_origin};
use frontend_api_dtos::{EventAccessAdministration, ParticipantAccessExplanation};
use frontend_test_support::fixtures::golden;
use frontend_transport::Error;
use serde_json::json;

fn access_golden() -> &'static str {
    golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access.json")
}
fn participants_golden() -> &'static str {
    golden!("GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access__participants.json")
}
const MISSION: &str = "89b1b731-37a8-4926-901a-3c7ff7de5eb3";

fn access() -> EventAccessAdministration {
    serde_json::from_str(access_golden()).unwrap()
}

fn participants() -> Vec<ParticipantAccessExplanation> {
    serde_json::from_str(participants_golden()).unwrap()
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
    assert!(
        slot_origin(
            &access,
            MISSION,
            "BLUFOR",
            "Bravo",
            "00000000-0000-4000-5000-000000000013"
        )
        .is_own()
    );
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

/* ───────────────────────── participant evidence ───────────────────────── */

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

/// A stale revision is told apart from every other conflict.
#[test]
fn a_stale_revision_is_told_apart_from_other_conflicts() {
    let stale = Error::from_error_body(
        409,
        Some(&json!({
            "error": "access settings changed since this form was loaded",
            "details": {"code": "ACCESS_REVISION_CONFLICT", "access_revision": 6}
        })),
    );
    assert!(is_revision_conflict(&stale));
    let referenced = Error::from_error_body(
        409,
        Some(
            &json!({"error": "an access policy still names this group; remove it from every policy first"}),
        ),
    );
    assert!(!is_revision_conflict(&referenced));
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
