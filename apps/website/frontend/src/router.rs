//! Route table — the single source of truth for the app's routes, mirroring router.tsx. It drives
//! the leptos_router `<Routes>` (T-159.4b) and is extracted for the S-routes gate (diffed against
//! tools/tbd-tools/fixtures/t159/manifests/routes.csv). Paths use the React shape ("/events/:id") so the
//! extracted manifest diffs byte-equal to the React oracle.
//!
//! T-805 — `auth` is enforced client-side (see [`required_role`] / [`role_may_enter`] /
//! [`auth_denial_redirect`]); the SPA still serves 200 for every path — the guard redirects after
//! mount, it does not change the server's catch-all.

use crate::shell::nav_config::{has_min_role_authed, Role};

/// One route. `auth` is the ProtectedRoute tier ("none" | "mission_maker" | "admin"); `full_bleed`
/// / `chromeless` are the route-handle layout flags.
// Consumed by the leptos_router <Routes> in T-159.4b; the S-routes extractor reads this table now.
#[allow(dead_code)]
pub struct RouteDef {
    pub path: &'static str,
    pub component: &'static str,
    pub full_bleed: bool,
    pub chromeless: bool,
    pub auth: &'static str,
}

#[allow(dead_code)]
pub static ROUTES: &[RouteDef] = &[
    RouteDef {
        path: "/login",
        component: "LoginPage",
        full_bleed: false,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/auth/callback",
        component: "AuthCallbackPage",
        full_bleed: false,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/",
        component: "DashboardPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/server-intel",
        component: "ServerIntelPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/announcements",
        component: "AnnouncementsPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/announcements/:id",
        component: "AnnouncementsPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/deployments",
        component: "DeploymentsPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/leaderboards",
        component: "LeaderboardsPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/missions",
        component: "MissionLibraryPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/missions/:id",
        component: "MissionOverviewPage",
        full_bleed: false,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/missions/:id/edit",
        component: "MissionEditorPage",
        full_bleed: true,
        chromeless: true,
        auth: "mission_maker",
    },
    RouteDef {
        path: "/events",
        component: "EventSchedulePage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/events/:id",
        component: "EventHubPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/events/:id/missions/:emid/orbat",
        component: "OrbatSelectionPage",
        full_bleed: false,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/wiki",
        component: "WikiPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/wiki/:slug",
        component: "WikiPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/vehicles",
        component: "VehicleDatabasePage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/modpacks",
        component: "ModpacksPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/tools/mortar",
        component: "MortarCalculatorPage",
        full_bleed: true,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/settings",
        component: "SettingsPage",
        full_bleed: false,
        chromeless: false,
        auth: "none",
    },
    RouteDef {
        path: "/admin/events",
        component: "EventManagerPage",
        full_bleed: false,
        chromeless: false,
        auth: "admin",
    },
    RouteDef {
        path: "/admin/approvals",
        component: "MissionApprovalsPage",
        full_bleed: true,
        chromeless: false,
        auth: "admin",
    },
    RouteDef {
        path: "/admin/server",
        component: "ServerControlPage",
        full_bleed: true,
        chromeless: false,
        auth: "admin",
    },
    RouteDef {
        path: "/admin/personnel",
        component: "PersonnelRosterPage",
        full_bleed: true,
        chromeless: false,
        auth: "admin",
    },
    RouteDef {
        path: "/admin/content",
        component: "ContentManagerPage",
        full_bleed: true,
        chromeless: false,
        auth: "admin",
    },
    RouteDef {
        path: "/admin/audit",
        component: "AuditLogsPage",
        full_bleed: true,
        chromeless: false,
        auth: "admin",
    },
    RouteDef {
        path: "*",
        component: "NotFoundPage",
        full_bleed: false,
        chromeless: false,
        auth: "none",
    },
];

/// Match a concrete path against the ROUTES table by segment (a `:param` segment is a wildcard),
/// returning the matched route. Resolves breadcrumb + full_bleed for dynamic routes.
fn match_route(path: &str) -> Option<&'static RouteDef> {
    fn seg_match(pattern: &str, path: &str) -> bool {
        if pattern == "*" {
            return false; // the catch-all is the <Routes fallback>, not a breadcrumb source
        }
        let ps: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let xs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        ps.len() == xs.len()
            && ps
                .iter()
                .zip(&xs)
                .all(|(p, x)| p.starts_with(':') || p == x)
    }
    ROUTES.iter().find(|r| seg_match(r.path, path))
}

/// Breadcrumb (parent, current) for a route — mirrors the router.tsx route handles, keyed on the
/// matched route pattern so dynamic routes resolve (/missions/abc → "Mission Overview"). TopNav
/// falls back to the plain title on `None`.
pub fn breadcrumb(path: &str) -> Option<(&'static str, &'static str)> {
    Some(match match_route(path)?.path {
        "/" => ("Command Center", "Dashboard"),
        "/server-intel" => ("Command Center", "Server Intel"),
        "/announcements" | "/announcements/:id" => ("Command Center", "Announcements"),
        "/deployments" => ("Operations", "My Deployments"),
        "/leaderboards" => ("Operations", "Global Leaderboards"),
        "/missions" => ("Mission Hub", "Mission Library"),
        "/missions/:id" => ("Mission Hub", "Mission Overview"),
        "/events" => ("Operations", "Event Schedule"),
        "/events/:id" => ("Operations", "Event Hub"),
        "/events/:id/missions/:emid/orbat" => ("Operations", "ORBAT Selection"),
        "/wiki" | "/wiki/:slug" => ("Doctrine & Info", "SOPs & Manuals"),
        "/vehicles" => ("Doctrine & Info", "Vehicle Database"),
        "/modpacks" => ("Doctrine & Info", "Modpacks"),
        "/tools/mortar" => ("Field Tools", "Mortar Calculator"),
        "/settings" => ("Account", "Settings"),
        "/admin/events" => ("Administration", "Event Manager"),
        "/admin/approvals" => ("Administration", "Mission Approvals"),
        "/admin/server" => ("Administration", "Server Control"),
        "/admin/personnel" => ("Administration", "Personnel Roster"),
        "/admin/content" => ("Administration", "Comms Broadcaster"),
        "/admin/audit" => ("Administration", "Audit Logs"),
        _ => return None,
    })
}

/// Whether a route is full-bleed (the `<main>` is `overflow-hidden` vs the padded scroll container),
/// via the matched route pattern (dynamic routes included). Unmatched defaults to false (padded),
/// matching react-router's no-handle case.
pub fn full_bleed(path: &str) -> bool {
    match_route(path).map(|r| r.full_bleed).unwrap_or(false)
}

/// Whether a route is chromeless (renders full-viewport with no Sidebar/TopNav — the Mission
/// Creator editor), from the route handle, via the matched route pattern.
pub fn chromeless(path: &str) -> bool {
    match_route(path).map(|r| r.chromeless).unwrap_or(false)
}

/// Declared RequireMinRole tier for `path`, or `None` when the route is open (`auth: "none"`).
/// T-805 — the table always declared this; the client now enforces it.
pub fn required_role(path: &str) -> Option<Role> {
    Role::from_route_auth(match_route(path)?.auth)
}

/// Action-gate: may this signed-in (or guest) role enter `path`?
/// Mirrors API `RequireMinRole` / [`has_min_role_authed`] — guests never pass a declared tier.
pub fn role_may_enter(path: &str, role: Option<Role>) -> bool {
    match required_role(path) {
        None => true,
        Some(min) => has_min_role_authed(role, min),
    }
}

/// Where to send a user who fails [`role_may_enter`].
///
/// Editor (`auth: "mission_maker"`) → mission overview with `?role_notice=mission_maker`.
/// Other declared tiers (admin pages) return `None` so existing `<AdminGate>` copy stays the
/// surface — T-805's defect was the editor only.
pub fn auth_denial_redirect(path: &str) -> Option<String> {
    let route = match_route(path)?;
    if route.auth != "mission_maker" {
        return None;
    }
    // `/missions/:id/edit` → `/missions/:id?role_notice=mission_maker`
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // missions / :id / edit
    if segments.len() == 3 && segments[0] == "missions" && segments[2] == "edit" {
        return Some(format!(
            "/missions/{}?role_notice={}",
            segments[1], route.auth
        ));
    }
    Some(format!("/missions?role_notice={}", route.auth))
}

#[cfg(test)]
mod t805_route_auth {
    use super::{auth_denial_redirect, required_role, role_may_enter, ROUTES};
    use crate::shell::nav_config::Role;

    #[test]
    fn editor_route_declares_mission_maker() {
        let edit = ROUTES
            .iter()
            .find(|r| r.path == "/missions/:id/edit")
            .expect("editor route in ROUTES");
        assert_eq!(edit.auth, "mission_maker");
        assert_eq!(
            required_role("/missions/abc-uuid/edit"),
            Some(Role::MissionMaker)
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

    #[test]
    fn open_routes_have_no_required_role() {
        assert_eq!(required_role("/missions"), None);
        assert_eq!(required_role("/missions/abc"), None);
        assert!(role_may_enter("/missions/abc", Some(Role::Enlisted)));
    }
}
