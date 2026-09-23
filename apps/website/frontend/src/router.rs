//! Route table — the single source of truth for the app's routes, mirroring router.tsx. It drives
//! the leptos_router `<Routes>` (T-159.4b) and is extracted for the S-routes gate (diffed against
//! tools_v2/developer-tools/fixtures/dom_oracle/manifests/routes.csv). Paths use the React shape ("/events/:id") so the
//! extracted manifest diffs byte-equal to the React oracle.
//!
//! Each route's `auth` tier is enforced client-side. [`role_may_enter`] decides whether a viewer's
//! role clears the matched route's declaration and refuses every viewer when the declaration is
//! not a recognised tier; [`auth_denial_redirect`] names where a refused viewer is sent (admin
//! pages have no redirect and render their own `<AdminGate>` refusal). The route guard in
//! `v2::core::auth::route_guard` applies both in the browser after mount; the server's catch-all
//! answers every path with 200 whatever its tier.

use crate::v2::core::auth::{has_min_role_authed, Role};

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
        // The Scenario Creator opened read-only on the version an artifact compiled from; the
        // backend serves it to the mission's author and to administrators.
        path: "/missions/:id/artifacts/:artifact_id/workspace",
        component: "ReviewWorkspacePage",
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
        // Debug bench — URL-only (no nav entry): blueprint viewer + interactive 2.5D LOS.
        path: "/debug/building-viewer",
        component: "BuildingViewerPage",
        full_bleed: true,
        chromeless: true,
        auth: "none",
    },
    RouteDef {
        // Debug bench — URL-only: the world occluder around a map point (T-090.12.5).
        path: "/debug/world-los",
        component: "WorldLosPage",
        full_bleed: true,
        chromeless: true,
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
        "/missions/:id/artifacts/:artifact_id/workspace" => ("Mission Hub", "Review Workspace"),
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

/// Whether a viewer holding `role` may enter `path`, judged by the matched route's `auth`
/// declaration.
///
/// An open route (`"none"`) and a path no route matches admit every viewer. A declared tier admits
/// a signed-in viewer whose role clears it under [`has_min_role_authed`], so a signed-in
/// [`Role::Guest`] clears a `"guest"` tier; an anonymous viewer (`None`: signed out, or a session
/// not yet bootstrapped) never clears a declared tier. An unrecognised declaration refuses every
/// viewer.
pub fn role_may_enter(path: &str, role: Option<Role>) -> bool {
    let Some(route) = match_route(path) else {
        return true;
    };
    route_auth_allows(route.auth, role)
}

/// Invalid route declarations fail closed for every account tier.
fn route_auth_allows(auth: &str, role: Option<Role>) -> bool {
    auth == "none"
        || Role::from_route_auth(auth).is_some_and(|minimum| has_min_role_authed(role, minimum))
}

/// Where the route guard sends a viewer that [`role_may_enter`] refuses, or `None` to leave the
/// viewer on the page.
///
/// A `mission_maker` route under `/missions/:id/` (the editor, the review workspace) returns that
/// mission's overview, `/missions/:id?role_notice=mission_maker`; any other `mission_maker` route
/// returns the library, `/missions?role_notice=mission_maker`. Every other route, and a path no
/// route matches, returns `None`. The admin pages need no redirect because each wraps its body in
/// `<AdminGate>`, which renders the refusal in place of the page.
pub fn auth_denial_redirect(path: &str) -> Option<String> {
    let route = match_route(path)?;
    if route.auth != "mission_maker" {
        return None;
    }
    // `/missions/:id/…` → `/missions/:id?role_notice=mission_maker`
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() >= 3 && segments[0] == "missions" {
        return Some(format!(
            "/missions/{}?role_notice={}",
            segments[1], route.auth
        ));
    }
    Some(format!("/missions?role_notice={}", route.auth))
}

#[cfg(test)]
#[path = "tests/route_authorization.rs"]
mod tests;
