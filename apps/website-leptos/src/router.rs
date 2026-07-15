//! Route table — the single source of truth for the app's routes, mirroring router.tsx. It drives
//! the leptos_router `<Routes>` (T-159.4b) and is extracted for the S-routes gate (diffed against
//! .ai/artifacts/t159_gates/manifests/routes.csv). Paths use the React shape ("/events/:id") so the
//! extracted manifest diffs byte-equal to the React oracle.

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
        "/announcements" => ("Command Center", "Announcements"),
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
