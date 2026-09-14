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
    // Pre-bootstrap / guest must not satisfy maker (or any) action affordances.
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
    assert_eq!(
        Role::from_route_auth("mission_maker"),
        Some(Role::MissionMaker)
    );
    assert_eq!(Role::from_route_auth("admin"), Some(Role::Admin));
    assert_eq!(Role::from_route_auth("none"), None);
    assert_eq!(Role::from_route_auth(""), None);
}
