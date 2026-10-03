//! Ending a session runs the registered sign-out hooks with the departing account's id, and runs
//! none when no account with an id is leaving.

#![cfg(not(target_arch = "wasm32"))]

use super::{register_logout_hook, run_logout_hooks};
use crate::foundation::auth::session::Session;
use crate::foundation::auth::AuthStore;
use crate::foundation::transport::dto::role::Role;
use crate::foundation::transport::dto::User;
use leptos::prelude::*;
use std::cell::RefCell;

thread_local! {
    /// What the recording hooks received, on this test's thread.
    static RECEIVED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn record(departing: &str) {
    RECEIVED.with(|received| received.borrow_mut().push(departing.to_string()));
}

fn record_second(departing: &str) {
    RECEIVED.with(|received| received.borrow_mut().push(format!("second:{departing}")));
}

fn received() -> Vec<String> {
    RECEIVED.with(|received| received.borrow().clone())
}

fn session(discord_id: &str) -> Session {
    Session {
        access_token: "header.e30.signature".into(),
        refresh_token: format!("refresh-{discord_id}"),
        expires_at: "2026-01-02T00:00:00Z".into(),
        user: User {
            discord_id: discord_id.into(),
            username: "member".into(),
            discord_handle: "member".into(),
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
        },
        arma_linked: false,
    }
}

fn with_store(test: impl FnOnce(AuthStore)) {
    let owner = Owner::new();
    owner.with(|| test(AuthStore::new()));
}

#[test]
fn a_registered_hook_receives_the_departing_account_id() {
    register_logout_hook(record);
    with_store(|store| {
        store.set_session(session("alice"));
        store.clear_session();
    });
    assert_eq!(received(), ["alice"]);
}

#[test]
fn a_blank_departing_id_runs_no_hook() {
    register_logout_hook(record);
    with_store(|store| {
        store.set_session(session("   "));
        store.clear_session();
    });
    assert!(received().is_empty(), "a blank id ran {:?}", received());
}

#[test]
fn ending_a_session_with_no_account_runs_no_hook() {
    register_logout_hook(record);
    with_store(|store| {
        store.set_session(session("alice"));
        store.clear_session();
        store.clear_session();
    });
    assert_eq!(
        received(),
        ["alice"],
        "the second end had no departing account"
    );
}

#[test]
fn hooks_run_in_registration_order() {
    register_logout_hook(record);
    register_logout_hook(record_second);
    run_logout_hooks("bob");
    assert_eq!(received(), ["bob", "second:bob"]);
}
