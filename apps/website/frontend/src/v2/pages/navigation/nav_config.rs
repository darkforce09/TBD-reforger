//! The sidebar's contents: which links exist, how they are grouped, and who may see them.
//!
//! **Role:** a single static registry of navigation sections and the links inside them. It is
//! data only — the rendering lives in `sidebar.rs` and the tier comparison in `core::auth`.
//! **Position:** read by the sidebar on every render, on desktop and in the mobile drawer alike.
//! **Signals & state:** none. [`NAVIGATION`] is a `'static` table with no interior mutability;
//! the viewer's tier arrives as an argument at render time.
//! **Invariants:** order is meaningful — sections render top to bottom and items in the order
//! written here. `path` values must match the route table, since the active-link rule compares
//! them against the live pathname by prefix. Exactly one section is marked `admin`, which gives
//! it both its own visibility check and its distinct framing.
//!
//! Every item currently declares [`Role::Enlisted`], so browse mode shows the first five sections
//! to everyone; only the Administration section, and its six items, ask for [`Role::Admin`].

use crate::v2::core::auth::Role;

/// One sidebar link.
///
/// * `label` — the visible text.
/// * `path` — the destination, matched against the live pathname to decide the active link.
/// * `icon` — the Material Symbols ligature rendered ahead of the label.
/// * `min_role` — the lowest tier that may see this link.
pub struct NavItem {
    pub label: &'static str,
    pub path: &'static str,
    pub icon: &'static str,
    pub min_role: Role,
}

/// A titled group of sidebar links.
///
/// * `title` — the group heading.
/// * `admin` — marks the privileged group: it is hidden below its tier and framed distinctly.
/// * `items` — the links, rendered in this order.
pub struct NavSection {
    pub title: &'static str,
    pub admin: bool,
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
            min_role: Role::Enlisted,
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
        ],
    },
];
