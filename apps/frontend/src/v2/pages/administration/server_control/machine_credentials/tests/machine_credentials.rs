//! The credential sheet's words and checks, held against the captured credential list.

use super::credential_text::*;
use crate::v2::core::api::dto::{ExecutorKind, MachineCredential, MachineCredentialList};
use crate::v2::core::test_support::fixtures::golden;

const LIST: &str = golden!("GET__servers__00000000-0000-4000-d000-000000000001__credentials.json");

fn credentials() -> Vec<MachineCredential> {
    serde_json::from_str::<MachineCredentialList>(LIST)
        .unwrap()
        .items
}

/// The captured live credential states its last use; the revoked one states who, when and why.
#[test]
fn captured_credentials_state_their_use_and_revocation() {
    let list = credentials();
    let live = &list[0];
    assert_eq!(
        standing_line(live, None),
        "Live · last used 2026-07-20 19:00 UTC"
    );
    assert_eq!(
        issued_line(live, None),
        "Issued by 000000000000000001, 2026-07-15 14:10 UTC"
    );
    let revoked = &list[1];
    assert_eq!(
        standing_line(revoked, Some("000000000000000001")),
        "Revoked by you, 2026-07-18 10:00 UTC: Rotated after the host rebuild"
    );
    assert_eq!(executor_label(&live.executor_kind), "Game runtime");
    assert_eq!(executor_label(&revoked.executor_kind), "Host agent");
    assert_eq!(executor_label("fleet_relay"), "fleet relay");
}

#[test]
fn a_credential_never_used_says_so() {
    let mut fresh = credentials()[0].clone();
    fresh.last_used_at = None;
    assert_eq!(standing_line(&fresh, None), "Live · never used");
}

/// Live credentials list above revoked ones, newest first within each, compared as instants. The
/// capture holds the game runtime's credential, the retired host agent's and the host agent's
/// replacement issued after the rebuild.
#[test]
fn live_credentials_list_first_and_newest_first() {
    let mut list = credentials();
    let mut newer = list[0].clone();
    newer.id = "00000000-0000-4000-e000-000000000009".into();
    newer.created_at = "2026-07-15T14:10:00.5Z".into();
    list.insert(0, list[1].clone());
    list.push(newer);
    let ordered: Vec<(bool, String)> = listing_order(list)
        .into_iter()
        .map(|c| (c.revoked_at.is_some(), c.created_at))
        .collect();
    assert_eq!(
        ordered,
        vec![
            (false, "2026-07-18T10:05:00Z".to_string()),
            (false, "2026-07-15T14:10:00.5Z".to_string()),
            (false, "2026-07-15T14:10:00Z".to_string()),
            (true, "2026-07-15T14:12:00Z".to_string()),
            (true, "2026-07-15T14:12:00Z".to_string()),
        ]
    );
}

/// The issue form offers exactly the two program kinds the backend takes, by their wire names.
#[test]
fn the_issue_form_offers_the_backend_program_kinds() {
    assert_eq!(executor_kind("host_agent"), Some(ExecutorKind::HostAgent));
    assert_eq!(executor_kind("mod_runtime"), Some(ExecutorKind::ModRuntime));
    assert_eq!(executor_kind("fleet_relay"), None);
    for (kind, wire, _) in EXECUTOR_KINDS {
        assert_eq!(serde_json::to_value(kind).unwrap(), serde_json::json!(wire));
    }
}

/// A label and a reason are trimmed and bounded as the backend bounds them, and a revocation
/// without a reason is refused before it is sent.
#[test]
fn labels_and_reasons_are_bounded_as_the_backend_bounds_them() {
    assert_eq!(
        validated_label("  Rack 2 host agent "),
        Ok("Rack 2 host agent".to_string())
    );
    assert!(validated_label("   ").is_err());
    assert!(validated_label(&"x".repeat(129)).is_err());
    assert!(
        validated_label(&"é".repeat(65)).is_err(),
        "130 bytes, though 65 characters"
    );
    assert_eq!(validated_reason(" Rotated "), Ok("Rotated".to_string()));
    assert!(validated_reason("\n\t").is_err());
    assert!(validated_reason(&"x".repeat(513)).is_err());
    assert_eq!(account_label("1", Some("1")), "you");
    assert_eq!(account_label("1", Some("2")), "1");
}
