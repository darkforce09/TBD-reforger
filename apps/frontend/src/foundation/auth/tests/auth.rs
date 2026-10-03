//! The persisted blob's shape and the route guard's decisions.

use super::session::{from_persist_json, to_persist_json, PersistState};
use super::*;
use crate::foundation::transport::dto::role::Role;
use crate::foundation::transport::dto::User;

fn sample_user() -> User {
    User {
        discord_id: "123".into(),
        username: "cpl".into(),
        discord_handle: "cpl#0001".into(),
        avatar_url: "https://cdn/a.png".into(),
        arma_id: None,
        arma_character: String::new(),
        role: Role::Admin,
        is_banned: false,
        ban_reason: String::new(),
        banned_by: None,
        banned_at: None,
        total_deployments: 5,
        attendance_rate: 0.9,
        last_login_at: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-02T00:00:00Z".into(),
    }
}

// The persisted blob's shape, exactly: the wrapper, the keys, and their spelling.
#[test]
fn persist_blob_shape_matches_tbd_auth() {
    let state = PersistState {
        session_id: None,
        refresh_token: Some("rt-abc".into()),
        user: Some(sample_user()),
        expires_at: Some("2026-01-01T01:00:00Z".into()),
    };
    let json = to_persist_json(&state);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(v["version"], 0);
    // Persist keys are camelCase…
    assert_eq!(v["state"]["refreshToken"], "rt-abc");
    assert_eq!(v["state"]["expiresAt"], "2026-01-01T01:00:00Z");
    // …and the access token is never persisted, which is the whole point of the slice.
    assert!(v["state"].get("accessToken").is_none());
    // …while the user object keeps the snake_case API contract + string role + null arma_id.
    assert_eq!(v["state"]["user"]["discord_id"], "123");
    assert_eq!(v["state"]["user"]["role"], "admin");
    assert_eq!(v["state"]["user"]["arma_id"], serde_json::Value::Null);
}

#[test]
fn persist_round_trips() {
    let state = PersistState {
        session_id: None,
        refresh_token: Some("rt".into()),
        user: Some(sample_user()),
        expires_at: Some("2026".into()),
    };
    let back = from_persist_json(&to_persist_json(&state)).unwrap();
    assert!(
        back == state,
        "persist → hydrate must round-trip losslessly"
    );
}

/// The lower tiers are redirected off the editor; mission makers and administrators stay.
#[test]
fn route_auth_redirect_blocks_enlisted_editor() {
    let path = "/missions/abc/edit";
    assert_eq!(
        route_auth_redirect(path, Some(Role::Enlisted), false).as_deref(),
        Some("/missions/abc?role_notice=mission_maker")
    );
    assert_eq!(
        route_auth_redirect(path, None, false).as_deref(),
        Some("/missions/abc?role_notice=mission_maker")
    );
    assert!(route_auth_redirect(path, Some(Role::MissionMaker), false).is_none());
    assert!(route_auth_redirect(path, Some(Role::Admin), false).is_none());
    // Deep-link safety: do not bounce while bootstrap is in flight.
    assert!(route_auth_redirect(path, None, true).is_none());
}

/// The browser store must actually install the route guard, not merely declare one.
#[test]
fn auth_store_new_installs_route_guard_on_wasm() {
    let src = crate::foundation::test_support::pins::auth_source();
    let src: &str = &src;
    assert!(
        src.contains("install_route_auth_guard(store)"),
        "AuthStore::new() must call install_route_auth_guard on wasm"
    );
    assert!(
        src.contains("route_auth_redirect"),
        "guard decision must go through route_auth_redirect"
    );
    assert!(
        src.contains("crate::foundation::route_table::role_may_enter"),
        "guard must reuse router RequireMinRole helpers"
    );
}
