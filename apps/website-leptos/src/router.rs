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
