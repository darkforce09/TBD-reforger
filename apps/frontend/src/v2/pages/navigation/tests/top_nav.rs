//! The account area's badge carries the profile fields it displays, and only those.

#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::v2::core::auth::Role;
use crate::v2::core::ui::DEFAULT_AVATAR;

fn member() -> User {
    User {
        discord_id: "100200300".into(),
        username: "member".into(),
        discord_handle: "member".into(),
        avatar_url: "https://cdn.discordapp.com/avatars/100200300/avatar.png".into(),
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

#[test]
fn a_linked_identity_shows_its_first_eight_characters() {
    let mut user = member();
    user.arma_id = Some("7656119800000001".into());
    assert_eq!(
        AccountBadge::of(&user).linked_identity_prefix.as_deref(),
        Some("76561198")
    );
}

#[test]
fn an_absent_or_empty_identity_counts_as_unlinked() {
    let mut user = member();
    assert_eq!(AccountBadge::of(&user).linked_identity_prefix, None);
    user.arma_id = Some(String::new());
    assert_eq!(AccountBadge::of(&user).linked_identity_prefix, None);
}

#[test]
fn the_avatar_goes_through_the_http_only_sink() {
    let mut user = member();
    assert_eq!(AccountBadge::of(&user).avatar, user.avatar_url);
    user.avatar_url = "javascript:alert(1)".into();
    assert_eq!(AccountBadge::of(&user).avatar, DEFAULT_AVATAR);
}

/// The account area renders from a memo of this badge, so a profile change the badge does not
/// display must produce an equal badge; otherwise every profile poll rebuilds the area.
#[test]
fn a_profile_change_the_badge_does_not_display_yields_an_equal_badge() {
    let before = member();
    let mut after = before.clone();
    after.last_login_at = Some("2026-01-01T12:00:00Z".into());
    after.attendance_rate = 0.5;
    after.total_deployments = 3;
    after.role = Role::Admin;
    assert!(AccountBadge::of(&before) == AccountBadge::of(&after));

    after.username = "renamed".into();
    assert!(AccountBadge::of(&before) != AccountBadge::of(&after));
}
