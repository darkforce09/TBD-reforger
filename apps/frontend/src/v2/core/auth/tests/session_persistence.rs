use super::{profile_persistence_update, PersistState, User};
use crate::v2::core::auth::Role;

fn user(discord_id: &str) -> User {
    User {
        discord_id: discord_id.into(),
        username: "Player".into(),
        discord_handle: "player".into(),
        avatar_url: String::new(),
        arma_id: None,
        arma_character: String::new(),
        role: Role::Guest,
        is_banned: false,
        ban_reason: String::new(),
        banned_by: None,
        banned_at: None,
        total_deployments: 0,
        attendance_rate: 0.0,
        last_login_at: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-01T00:00:00Z".into(),
    }
}

fn state(token: &str, account: Option<&str>, expiry: &str) -> PersistState {
    PersistState {
        session_id: None,
        refresh_token: Some(token.into()),
        user: account.map(user),
        expires_at: Some(expiry.into()),
    }
}

#[test]
fn profile_persistence_rejects_response_from_before_refresh_rotation() {
    let current = state("R2", Some("account-a"), "new-expiry");
    let proposed = state("R1", Some("account-a"), "old-expiry");
    assert!(profile_persistence_update(&current, &proposed).is_none());
    assert_eq!(current.refresh_token.as_deref(), Some("R2"));
    assert_eq!(current.expires_at.as_deref(), Some("new-expiry"));
}

#[test]
fn profile_persistence_rejects_a_different_account_even_with_matching_refresh() {
    let current = state("R1", Some("account-b"), "current-expiry");
    let proposed = state("R1", Some("account-a"), "proposed-expiry");
    assert!(profile_persistence_update(&current, &proposed).is_none());
}

#[test]
fn profile_persistence_accepts_initial_profile_for_the_current_refresh() {
    let current = state("R1", None, "current-expiry");
    let proposed = state("R1", Some("account-a"), "proposed-expiry");
    let updated = profile_persistence_update(&current, &proposed).expect("initial profile");
    assert!(updated.user == proposed.user);
    assert_eq!(updated.refresh_token, current.refresh_token);
    assert_eq!(updated.expires_at, current.expires_at);
}

#[test]
fn profile_persistence_updates_only_profile_and_preserves_current_expiry_and_refresh() {
    let current = state("R2", Some("account-a"), "new-expiry");
    let mut proposed = state("R2", Some("account-a"), "old-expiry");
    let profile = proposed.user.as_mut().unwrap();
    profile.username = "Updated Player".into();
    profile.role = Role::Admin;
    profile.total_deployments = 12;
    let updated = profile_persistence_update(&current, &proposed).expect("same session profile");
    assert!(updated.user == proposed.user);
    assert_eq!(updated.refresh_token, current.refresh_token);
    assert_eq!(updated.expires_at, current.expires_at);
}

#[test]
fn profile_persistence_requires_nonempty_refresh_credentials_and_a_profile() {
    let valid = state("R1", Some("account-a"), "current-expiry");
    for missing in [None, Some(String::new())] {
        let mut invalid = valid.clone();
        invalid.refresh_token = missing;
        assert!(profile_persistence_update(&invalid, &valid).is_none());
        assert!(profile_persistence_update(&valid, &invalid).is_none());
        assert!(profile_persistence_update(&invalid, &invalid).is_none());
    }
    let no_profile = state("R1", None, "current-expiry");
    assert!(profile_persistence_update(&valid, &no_profile).is_none());
    assert!(profile_persistence_update(&no_profile, &no_profile).is_none());
}

#[test]
fn logout_matches_a_rotated_successor_by_session_and_preserves_a_new_login() {
    let mut departing = state("R1", Some("account-a"), "old");
    departing.session_id = Some("session-one".into());
    let mut current = state("R2", Some("account-a"), "new");
    current.session_id = Some("session-one".into());
    assert!(super::persisted_belongs_to_session(&current, &departing));
    current.session_id = Some("session-two".into());
    assert!(!super::persisted_belongs_to_session(&current, &departing));
}

#[test]
fn legacy_credentials_match_only_the_exact_nonempty_token() {
    let departing = state("R1", Some("account-a"), "old");
    assert!(super::persisted_belongs_to_session(&departing, &departing));
    let successor = state("R2", Some("account-a"), "new");
    assert!(!super::persisted_belongs_to_session(&successor, &departing));
    let blank = state("", Some("account-a"), "old");
    assert!(!super::persisted_belongs_to_session(&blank, &blank));
}

#[test]
fn profile_storage_rejects_a_different_session_even_for_the_same_account() {
    let mut old = state("R1", Some("account-a"), "old");
    old.session_id = Some("session-one".into());
    let mut next = old.clone();
    next.session_id = Some("session-two".into());
    assert!(profile_persistence_update(&next, &old).is_none());
}

#[test]
fn profile_persistence_merges_after_a_peer_rotation_of_the_same_session() {
    let mut current = state("R2", Some("account-a"), "new-expiry");
    current.session_id = Some("session-one".into());
    let mut proposed = state("R1", Some("account-a"), "old-expiry");
    proposed.session_id = current.session_id.clone();
    proposed.user.as_mut().unwrap().username = "New profile".into();
    let updated =
        profile_persistence_update(&current, &proposed).expect("same session across rotation");
    assert_eq!(updated.refresh_token.as_deref(), Some("R2"));
    assert_eq!(updated.expires_at.as_deref(), Some("new-expiry"));
    assert_eq!(updated.user.unwrap().username, "New profile");
}
