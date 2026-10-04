//! The sidebar's contents: which links exist, how they are grouped, and who may see them.
//!
//! **Role:** a single static registry of navigation sections and the links inside them. It is
//! data only — the rendering lives in the app shell's sidebar and the tier comparison in the role
//! ladder of `frontend_api_dtos::role`.
//! **Position:** beside the route table ([`crate::routes`]); read by the app shell's sidebar on
//! every render, on desktop and in the mobile drawer alike.
//! **Signals & state:** none. [`NAVIGATION`] is a `'static` table with no interior mutability;
//! the viewer's tier arrives as an argument at render time.
//! **Invariants:** order is meaningful — sections render top to bottom and items in the order
//! written here. `path` values must match the route table, since the active-link rule compares
//! them against the live pathname by prefix. Exactly one section is marked `admin`, which gives
//! it both its own visibility check and its distinct framing.
//!
//! Every item declares [`Role::Enlisted`] except the public Mortar Calculator, which declares
//! [`Role::Guest`] so a signed-in guest sees it too; browse mode shows the first five sections to
//! everyone; only the Administration section, and its seven items, ask for [`Role::Admin`].

use frontend_api_dtos::role::Role;

/// One sidebar link.
pub struct NavItem {
    /// The visible text.
    pub label: &'static str,
    /// The destination, matched against the live pathname to decide the active link.
    pub path: &'static str,
    /// The Material Symbols ligature rendered ahead of the label.
    pub icon: &'static str,
    /// The lowest tier that may see this link.
    pub min_role: Role,
}

/// A titled group of sidebar links.
pub struct NavSection {
    /// The group heading.
    pub title: &'static str,
    /// Marks the privileged group: it is hidden below its tier and framed distinctly.
    pub admin: bool,
    /// The links, rendered in this order.
    pub items: &'static [NavItem],
}

/// The sidebar, top to bottom: the six command hubs and the links inside each.
pub static NAVIGATION: &[NavSection] = &[
    NavSection {
        title: "Command Center",
        admin: false,
        items: &[
            NavItem {
                label: "Dashboard",
                path: "/",
                icon: "grid_view",
                min_role: Role::Enlisted,
            },
            NavItem {
                label: "Server Intel",
                path: "/server-intel",
                icon: "dns",
                min_role: Role::Enlisted,
            },
            NavItem {
                label: "Announcements",
                path: "/announcements",
                icon: "campaign",
                min_role: Role::Enlisted,
            },
        ],
    },
    NavSection {
        title: "Operations",
        admin: false,
        items: &[
            NavItem {
                label: "Event Schedule",
                path: "/events",
                icon: "calendar_month",
                min_role: Role::Enlisted,
            },
            NavItem {
                label: "My Deployments",
                path: "/deployments",
                icon: "military_tech",
                min_role: Role::Enlisted,
            },
            NavItem {
                label: "Global Leaderboards",
                path: "/leaderboards",
                icon: "leaderboard",
                min_role: Role::Enlisted,
            },
        ],
    },
    NavSection {
        title: "Mission Hub",
        admin: false,
        items: &[NavItem {
            label: "Mission Library",
            path: "/missions",
            icon: "library_books",
            min_role: Role::Enlisted,
        }],
    },
    NavSection {
        title: "Field Tools",
        admin: false,
        items: &[NavItem {
            label: "Mortar Calculator",
            path: "/tools/mortar",
            icon: "calculate",
            min_role: Role::Guest,
        }],
    },
    NavSection {
        title: "Doctrine & Info",
        admin: false,
        items: &[
            NavItem {
                label: "SOPs & Manuals",
                path: "/wiki",
                icon: "menu_book",
                min_role: Role::Enlisted,
            },
            NavItem {
                label: "Vehicle Database",
                path: "/vehicles",
                icon: "directions_car",
                min_role: Role::Enlisted,
            },
            NavItem {
                label: "Modpacks",
                path: "/modpacks",
                icon: "extension",
                min_role: Role::Enlisted,
            },
        ],
    },
    NavSection {
        title: "Administration",
        admin: true,
        items: &[
            NavItem {
                label: "Event Manager",
                path: "/admin/events",
                icon: "event_available",
                min_role: Role::Admin,
            },
            NavItem {
                label: "Mission Approvals",
                path: "/admin/approvals",
                icon: "fact_check",
                min_role: Role::Admin,
            },
            NavItem {
                label: "Server Control",
                path: "/admin/server",
                icon: "settings_system_daydream",
                min_role: Role::Admin,
            },
            NavItem {
                label: "Personnel Roster",
                path: "/admin/personnel",
                icon: "groups",
                min_role: Role::Admin,
            },
            NavItem {
                label: "Comms Broadcaster",
                path: "/admin/content",
                icon: "campaign",
                min_role: Role::Admin,
            },
            NavItem {
                label: "Audit Logs",
                path: "/admin/audit",
                icon: "receipt_long",
                min_role: Role::Admin,
            },
            NavItem {
                label: "Ballistics Catalogs",
                path: "/admin/ballistics-catalogs",
                icon: "track_changes",
                min_role: Role::Admin,
            },
        ],
    },
];
