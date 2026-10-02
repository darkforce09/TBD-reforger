use super::{auth_denial_redirect, match_route, role_may_enter, route_auth_allows, ROUTES};
use crate::v2::core::auth::Role;

/// Every viewer a route can meet: an anonymous (or not yet bootstrapped) visitor and each tier on
/// the role ladder.
const EVERY_VIEWER: [Option<Role>; 6] = [
    None,
    Some(Role::Guest),
    Some(Role::Enlisted),
    Some(Role::Leader),
    Some(Role::MissionMaker),
    Some(Role::Admin),
];

#[test]
fn editor_route_declares_mission_maker() {
    let edit = ROUTES
        .iter()
        .find(|r| r.path == "/missions/:id/edit")
        .expect("editor route in ROUTES");
    assert_eq!(edit.auth, "mission_maker");
    assert_eq!(
        match_route("/missions/abc-uuid/edit").map(|r| r.auth),
        Some("mission_maker")
    );
}

#[test]
fn enlisted_blocked_maker_and_admin_pass() {
    let path = "/missions/1877c175-0000-0000-0000-000000000001/edit";
    assert!(
        !role_may_enter(path, Some(Role::Enlisted)),
        "enlisted must not enter the editor"
    );
    assert!(
        !role_may_enter(path, Some(Role::Leader)),
        "leader is below mission_maker"
    );
    assert!(
        !role_may_enter(path, None),
        "guest must not enter (has_min_role_authed None=>false)"
    );
    assert!(role_may_enter(path, Some(Role::MissionMaker)));
    assert!(role_may_enter(path, Some(Role::Admin)));
}

#[test]
fn denial_redirects_to_overview_with_role_notice() {
    let dest = auth_denial_redirect("/missions/abc/edit").expect("editor denial target");
    assert_eq!(dest, "/missions/abc?role_notice=mission_maker");
    assert!(
        auth_denial_redirect("/missions/abc").is_none(),
        "open routes have no denial redirect"
    );
    assert!(
        auth_denial_redirect("/admin/events").is_none(),
        "admin pages keep AdminGate — no redirect from this helper"
    );
}

/// The mission library and a mission's overview declare the open tier, and every viewer, signed in
/// or not, may enter them.
#[test]
fn open_routes_declare_none_and_admit_every_viewer() {
    for path in ["/missions", "/missions/abc"] {
        assert_eq!(match_route(path).map(|r| r.auth), Some("none"));
        for viewer in EVERY_VIEWER {
            assert!(role_may_enter(path, viewer), "{viewer:?} must enter {path}");
        }
    }
}

#[test]
fn unknown_route_auth_never_grants_access_and_guest_is_authenticated() {
    for role in [
        None,
        Some(Role::Guest),
        Some(Role::Enlisted),
        Some(Role::Admin),
    ] {
        for invalid in ["", "administrator", "Admin", " guest", "unknown"] {
            assert!(!route_auth_allows(invalid, role));
        }
    }
    assert!(!route_auth_allows("guest", None));
    assert!(route_auth_allows("guest", Some(Role::Guest)));
    assert!(route_auth_allows("none", None));
}

/// A declaration that is not exactly `"none"` or a tier's spelling admits nobody: a near miss of a
/// tier never admits the tier it was meant to name, and a near miss of `"none"` never opens the
/// route.
#[test]
fn a_misdeclared_tier_denies_every_viewer() {
    for declaration in [
        "",
        "mision_maker",
        "mission-maker",
        "MissionMaker",
        "admin ",
        " admin",
        "ADMIN",
        "enlisted\n",
        "leader\t",
        "gust",
        "none ",
        "None",
        "public",
    ] {
        for viewer in EVERY_VIEWER {
            assert!(
                !route_auth_allows(declaration, viewer),
                "{declaration:?} must not admit {viewer:?}"
            );
        }
    }
}

/// Every route declares the open tier or a tier on the role ladder. `role_may_enter` refuses every
/// viewer on an unrecognised declaration, so a misspelling fails here instead of locking the page
/// for everyone at runtime.
#[test]
fn every_route_declares_a_recognised_tier() {
    for route in ROUTES {
        assert!(
            route.auth == "none" || Role::from_route_auth(route.auth).is_some(),
            "{} declares the unrecognised tier {:?}",
            route.path,
            route.auth
        );
    }
}

/// The review workspace needs the mission-maker tier the editor needs, and a viewer below it is sent
/// back to the mission's overview.
#[test]
fn the_review_workspace_declares_mission_maker_and_redirects_to_its_mission() {
    let path = "/missions/m-1/artifacts/a-1/workspace";
    assert_eq!(match_route(path).map(|r| r.auth), Some("mission_maker"));
    assert!(!role_may_enter(path, Some(Role::Enlisted)));
    assert!(!role_may_enter(path, None));
    assert!(role_may_enter(path, Some(Role::MissionMaker)));
    assert!(role_may_enter(path, Some(Role::Admin)));
    assert_eq!(
        auth_denial_redirect(path).as_deref(),
        Some("/missions/m-1?role_notice=mission_maker")
    );
    let workspace = ROUTES
        .iter()
        .find(|r| r.path == "/missions/:id/artifacts/:artifact_id/workspace")
        .expect("the review workspace route");
    assert!(workspace.chromeless && workspace.full_bleed);
}
