//! Ladder ordering and the two gates' opposite treatment of a viewer with no tier.

use super::{has_min_role, has_min_role_authed, Role};

#[test]
fn browse_mode_none_sees_all_nav() {
    // Intentional: unauthenticated browse-mode keeps every nav item visible.
    assert!(has_min_role(None, Role::Admin));
    assert!(has_min_role(None, Role::MissionMaker));
}

#[test]
fn action_gate_none_is_never_authorized() {
    // Anonymous / pre-bootstrap must not satisfy any authenticated action affordance.
    assert!(!has_min_role_authed(None, Role::Guest));
    assert!(!has_min_role_authed(None, Role::MissionMaker));
    assert!(!has_min_role_authed(None, Role::Enlisted));
    assert!(has_min_role_authed(
        Some(Role::MissionMaker),
        Role::MissionMaker
    ));
    assert!(has_min_role_authed(Some(Role::Admin), Role::MissionMaker));
    assert!(!has_min_role_authed(
        Some(Role::Enlisted),
        Role::MissionMaker
    ));
}

#[test]
fn from_route_auth_parses_declared_tiers() {
    assert_eq!(Role::from_route_auth("guest"), Some(Role::Guest));
    assert_eq!(Role::from_route_auth("enlisted"), Some(Role::Enlisted));
    assert_eq!(Role::from_route_auth("leader"), Some(Role::Leader));
    assert_eq!(
        Role::from_route_auth("mission_maker"),
        Some(Role::MissionMaker)
    );
    assert_eq!(Role::from_route_auth("admin"), Some(Role::Admin));
    assert_eq!(Role::from_route_auth("none"), None);
    assert_eq!(Role::from_route_auth(""), None);
}

#[test]
fn authenticated_guest_is_distinct_from_anonymous_and_below_members() {
    let roles = [
        Role::Guest,
        Role::Enlisted,
        Role::Leader,
        Role::MissionMaker,
        Role::Admin,
    ];
    for (user_index, user) in roles.iter().copied().enumerate() {
        for (required_index, required) in roles.iter().copied().enumerate() {
            assert_eq!(
                has_min_role_authed(Some(user), required),
                user_index >= required_index
            );
            assert_eq!(
                has_min_role(Some(user), required),
                user_index >= required_index
            );
        }
    }
    assert!(has_min_role_authed(Some(Role::Guest), Role::Guest));
    assert!(!has_min_role_authed(None, Role::Guest));
}

#[test]
fn guest_uses_the_api_wire_name() {
    assert_eq!(Role::Guest.as_str(), "guest");
    assert_eq!(serde_json::to_string(&Role::Guest).unwrap(), "\"guest\"");
    assert_eq!(
        serde_json::from_str::<Role>("\"guest\"").unwrap(),
        Role::Guest
    );
    assert!(serde_json::from_str::<Role>("\"anonymous\"").is_err());
}
