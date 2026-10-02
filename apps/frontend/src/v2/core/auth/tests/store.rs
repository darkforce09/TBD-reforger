//! Native signal tests for asynchronous profile ownership, response ordering, and which session
//! signals a profile adoption notifies.

#![cfg(not(target_arch = "wasm32"))]

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn user(id: &str) -> User {
    User {
        discord_id: id.into(),
        username: format!("member-{id}"),
        discord_handle: id.into(),
        avatar_url: String::new(),
        arma_id: None,
        arma_character: String::new(),
        role: Role::Enlisted,
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

fn access(id: &str, session: u32) -> String {
    use base64::Engine;
    let claims =
        serde_json::json!({"sub": id, "sid": format!("00000000-0000-0000-0000-{session:012x}")});
    format!(
        "header.{}.signature",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims.to_string())
    )
}

fn session(id: &str) -> Session {
    Session {
        access_token: access(id, 1),
        refresh_token: format!("refresh-{id}"),
        expires_at: "2026-01-02T00:00:00Z".into(),
        user: user(id),
        arma_linked: false,
    }
}

fn profile(id: &str, username: &str) -> MeResponse {
    let mut user = user(id);
    user.username = username.into();
    MeResponse {
        user,
        arma_linked: false,
        membership_stale: Some(true),
        membership_override_active: Some(true),
        can_manage_membership_override: Some(true),
    }
}

fn with_store(test: impl FnOnce(AuthStore)) {
    let owner = Owner::new();
    owner.with(|| test(AuthStore::new()));
}

fn assert_membership_clear(store: AuthStore) {
    assert!(!store.membership_stale.get_untracked());
    assert!(!store.membership_override_active.get_untracked());
    assert!(!store.can_manage_membership_override.get_untracked());
}

/// One subscriber per public session signal, each counting how often it ran.
struct SessionNotifications {
    counters: Vec<(&'static str, Arc<AtomicUsize>)>,
    _subscribers: Vec<ImmediateEffect>,
}

impl SessionNotifications {
    fn observe(store: AuthStore) -> Self {
        let mut observed = Self {
            counters: Vec::new(),
            _subscribers: Vec::new(),
        };
        observed.watch("user", store.user);
        observed.watch("access_token", store.access_token);
        observed.watch("refresh_token", store.refresh_token);
        observed.watch("expires_at", store.expires_at);
        observed.watch("bootstrapping", store.bootstrapping);
        observed.watch("membership_stale", store.membership_stale);
        observed.watch(
            "membership_override_active",
            store.membership_override_active,
        );
        observed.watch(
            "can_manage_membership_override",
            store.can_manage_membership_override,
        );
        observed
    }

    fn watch<T: Send + Sync + 'static>(&mut self, name: &'static str, signal: RwSignal<T>) {
        let runs = Arc::new(AtomicUsize::new(0));
        let subscriber = ImmediateEffect::new_isomorphic({
            let runs = Arc::clone(&runs);
            move || {
                signal.track();
                runs.fetch_add(1, Ordering::SeqCst);
            }
        });
        self.counters.push((name, runs));
        self._subscribers.push(subscriber);
    }

    /// How often each signal notified since it was observed; the subscriber's first run is not
    /// a notification.
    fn since_observed(&self) -> Vec<(&'static str, usize)> {
        self.counters
            .iter()
            .map(|(name, runs)| (*name, runs.load(Ordering::SeqCst) - 1))
            .collect()
    }
}

#[test]
fn adopting_an_unchanged_profile_notifies_no_session_signal() {
    with_store(|store| {
        store.set_session(session("alice"));
        let first = store.begin_profile_request();
        assert!(store.adopt_profile(first, &profile("alice", "current")));
        let notifications = SessionNotifications::observe(store);

        let poll = store.begin_profile_request();
        assert!(store.adopt_profile(poll, &profile("alice", "current")));

        for (signal, count) in notifications.since_observed() {
            assert_eq!(count, 0, "an unchanged profile notified `{signal}`");
        }
    });
}

#[test]
fn adopting_a_changed_profile_notifies_each_changed_signal_once() {
    with_store(|store| {
        store.set_session(session("alice"));
        let first = store.begin_profile_request();
        assert!(store.adopt_profile(first, &profile("alice", "current")));
        let notifications = SessionNotifications::observe(store);

        let mut renamed = profile("alice", "renamed");
        renamed.membership_stale = Some(false);
        let poll = store.begin_profile_request();
        assert!(store.adopt_profile(poll, &renamed));

        assert_eq!(store.user.get_untracked().unwrap().username, "renamed");
        assert!(!store.membership_stale.get_untracked());
        for (signal, count) in notifications.since_observed() {
            let expected = usize::from(matches!(signal, "user" | "membership_stale"));
            assert_eq!(count, expected, "`{signal}` notified {count} times");
        }
    });
}

#[test]
fn profile_response_cannot_restore_a_logged_out_session() {
    with_store(|store| {
        store.set_session(session("alice"));
        let initial = store.begin_profile_request();
        assert!(store.adopt_profile(initial, &profile("alice", "current")));
        let late = store.begin_profile_request();
        let generation = store.current_generation();

        store.clear_session();

        assert!(!store.is_current_generation(generation));
        assert!(!store.adopt_profile(late, &profile("alice", "obsolete")));
        assert!(store.user.get_untracked().is_none());
        assert!(store.access_token.get_untracked().is_none());
        assert_membership_clear(store);
    });
}

#[test]
fn profile_response_cannot_cross_an_account_switch() {
    with_store(|store| {
        store.set_session(session("alice"));
        let initial = store.begin_profile_request();
        assert!(store.adopt_profile(initial, &profile("alice", "current")));
        let late = store.begin_profile_request();
        let generation = store.current_generation();

        store.set_session(session("bob"));

        assert!(!store.is_current_generation(generation));
        assert_membership_clear(store);
        assert!(!store.adopt_profile(late, &profile("alice", "obsolete")));
        assert_eq!(store.user.get_untracked().unwrap().discord_id, "bob");
        assert_eq!(
            store.access_token.get_untracked().as_deref(),
            Some(access("bob", 1).as_str())
        );
    });
}

#[test]
fn profile_request_supersedes_older_responses_before_and_after_completion() {
    with_store(|store| {
        store.set_session(session("alice"));
        let older = store.begin_profile_request();
        let newer = store.begin_profile_request();

        assert!(!store.adopt_profile(older, &profile("alice", "older")));
        assert_eq!(store.user.get_untracked().unwrap().username, "member-alice");
        assert!(store.adopt_profile(newer, &profile("alice", "newer")));
        assert!(!store.adopt_profile(older, &profile("alice", "older")));
        assert_eq!(store.user.get_untracked().unwrap().username, "newer");
    });
}

#[test]
fn profile_response_for_the_wrong_account_is_rejected() {
    with_store(|store| {
        store.set_session(session("alice"));
        let request = store.begin_profile_request();

        assert!(!store.adopt_profile(request, &profile("bob", "unrelated")));
        assert_eq!(store.user.get_untracked().unwrap().discord_id, "alice");
        assert_membership_clear(store);
    });
}

#[test]
fn token_rotation_preserves_in_flight_profile_ownership() {
    with_store(|store| {
        store.set_session(session("alice"));
        let request = store.begin_profile_request();
        let generation = store.current_generation();

        store.set_tokens(RefreshResponse {
            access_token: access("alice", 1),
            refresh_token: "rotated-refresh".into(),
            expires_at: "2026-01-03T00:00:00Z".into(),
        });

        assert!(store.is_current_generation(generation));
        assert!(store.adopt_profile(request, &profile("alice", "after rotation")));
        assert_eq!(
            store.user.get_untracked().unwrap().username,
            "after rotation"
        );
        assert_eq!(
            store.access_token.get_untracked().as_deref(),
            Some(access("alice", 1).as_str())
        );
        assert!(store.membership_stale.get_untracked());
        assert!(store.membership_override_active.get_untracked());
        assert!(store.can_manage_membership_override.get_untracked());
    });
}

#[test]
fn profile_adoption_requires_an_access_token_and_allows_initial_profile() {
    with_store(|store| {
        let unauthenticated = store.begin_profile_request();
        assert!(!store.adopt_profile(unauthenticated, &profile("alice", "no token")));
        assert!(store.user.get_untracked().is_none());

        store.set_tokens(RefreshResponse {
            access_token: access("alice", 1),
            refresh_token: "bootstrap-refresh".into(),
            expires_at: "2026-01-03T00:00:00Z".into(),
        });
        let request = store.begin_profile_request();
        assert!(store.adopt_profile(request, &profile("alice", "restored")));
        assert_eq!(store.user.get_untracked().unwrap().discord_id, "alice");
    });
}

#[test]
fn a_new_session_for_the_same_account_rejects_previous_profile_requests() {
    with_store(|store| {
        store.set_session(session("alice"));
        let old = store.begin_profile_request();

        store.set_session(session("alice"));

        assert!(!store.adopt_profile(old, &profile("alice", "previous session")));
        assert_eq!(store.user.get_untracked().unwrap().username, "member-alice");
    });
}

#[test]
fn adopting_a_new_session_pair_for_the_same_account_invalidates_old_profile_work() {
    with_store(|store| {
        store.set_session(session("alice"));
        let old = store.begin_profile_request();
        let generation = store.current_generation();
        store.set_tokens(RefreshResponse {
            access_token: access("alice", 2),
            refresh_token: "new-session-refresh".into(),
            expires_at: "2026-01-03T00:00:00Z".into(),
        });
        assert!(!store.is_current_generation(generation));
        assert!(!store.adopt_profile(old, &profile("alice", "obsolete")));
        assert!(store.user.get_untracked().is_none());
        assert_membership_clear(store);
        let current = store.begin_profile_request();
        assert!(store.adopt_profile(current, &profile("alice", "new session")));
    });
}

#[test]
fn bootstrap_restores_session_identity_and_keeps_generation_during_its_rotation() {
    with_store(|store| {
        let mut saved = store.persist_state();
        saved.session_id = Some("00000000-0000-0000-0000-000000000001".into());
        saved.refresh_token = Some("original".into());
        saved.user = Some(user("alice"));
        store.restore_persisted(saved);
        let request = store.begin_profile_request();
        let generation = store.current_generation();
        store.set_tokens(RefreshResponse {
            access_token: access("alice", 1),
            refresh_token: "successor".into(),
            expires_at: "2026-01-03T00:00:00Z".into(),
        });
        assert!(store.is_current_generation(generation));
        assert!(store.adopt_profile(request, &profile("alice", "restored")));
    });
}
