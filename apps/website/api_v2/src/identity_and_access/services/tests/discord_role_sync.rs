use super::*;

// ── Empty stored snowflakes = no snapshot (the `Unavailable` mirror) ──

#[test]
fn empty_stored_roles_are_no_snapshot_not_no_roles() {
    // The admin-lockout shape: resync treating an empty `user_discord_roles` fetch as an
    // authoritative empty list → resolve_role → Enlisted. Absence of stored snowflakes is
    // Unavailable — skip, do not demote.
    assert!(
        resync_ids_from_snapshot(&[]).is_none(),
        "no stored Discord roles must not be an authoritative empty list"
    );
}

#[test]
fn non_empty_stored_roles_remain_authoritative_for_resolve() {
    // Guard the happy path: users who OAuth'd and hold snowflakes must still
    // reach resolve_role (remap / demote-when-unmapped still works).
    let ids = vec!["1517".into(), "8899".into()];
    assert_eq!(
        resync_ids_from_snapshot(&ids).expect("authoritative"),
        ["1517", "8899"]
    );
}

#[test]
fn single_unmapped_snowflake_is_still_a_snapshot() {
    // One stored id with no discord_roles mapping still resolves (to enlisted)
    // — that is a real OAuth snapshot, not the never-logged-in case.
    let ids = vec!["999999999999999999".into()];
    assert!(
        resync_ids_from_snapshot(&ids).is_some(),
        "a non-empty stored list is a snapshot even when nothing maps"
    );
}
