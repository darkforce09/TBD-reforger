//! Navigation model + role gate — ported from config/navigation.ts + lib/roles.ts.
//! Field-for-field identical to the React nav so the Sidebar renders the same items in the same
//! order (the S-components / V-shell gates check this).

#[derive(Clone, Copy, PartialEq, Eq)]
// The full four-tier ladder; Leader/MissionMaker aren't referenced by nav filtering yet (guest
// browse-mode shows all) — they wire in with the real auth store at T-159.3.
#[allow(dead_code)]
pub enum Role {
    Enlisted,
    Leader,
    MissionMaker,
    Admin,
}

impl Role {
    fn rank(self) -> u8 {
        match self {
            Role::Enlisted => 1,
            Role::Leader => 2,
            Role::MissionMaker => 3,
            Role::Admin => 4,
        }
    }
}

/// Browse mode: unauthenticated users (`None`) see all nav (mirrors lib/roles.ts `hasMinRole`).
pub fn has_min_role(user: Option<Role>, min: Role) -> bool {
    match user {
        None => true,
        Some(r) => r.rank() >= min.rank(),
    }
}

pub struct NavItem {
    pub label: &'static str,
    pub path: &'static str,
    pub icon: &'static str,
    pub min_role: Role,
}

pub struct NavSection {
    pub title: &'static str,
    pub admin: bool,
    pub items: &'static [NavItem],
}

pub static NAVIGATION: &[NavSection] = &[
    NavSection {
        title: "Command Center",
        admin: false,
        items: &[
            NavItem { label: "Dashboard", path: "/", icon: "grid_view", min_role: Role::Enlisted },
            NavItem { label: "Server Intel", path: "/server-intel", icon: "dns", min_role: Role::Enlisted },
            NavItem { label: "Announcements", path: "/announcements", icon: "campaign", min_role: Role::Enlisted },
        ],
    },
    NavSection {
        title: "Operations",
        admin: false,
        items: &[
            NavItem { label: "Event Schedule", path: "/events", icon: "calendar_month", min_role: Role::Enlisted },
            NavItem { label: "My Deployments", path: "/deployments", icon: "military_tech", min_role: Role::Enlisted },
            NavItem { label: "Global Leaderboards", path: "/leaderboards", icon: "leaderboard", min_role: Role::Enlisted },
        ],
    },
    NavSection {
        title: "Mission Hub",
        admin: false,
        items: &[NavItem { label: "Mission Library", path: "/missions", icon: "library_books", min_role: Role::Enlisted }],
    },
    NavSection {
        title: "Field Tools",
        admin: false,
        items: &[NavItem { label: "Mortar Calculator", path: "/tools/mortar", icon: "calculate", min_role: Role::Enlisted }],
    },
    NavSection {
        title: "Doctrine & Info",
        admin: false,
        items: &[
            NavItem { label: "SOPs & Manuals", path: "/wiki", icon: "menu_book", min_role: Role::Enlisted },
            NavItem { label: "Vehicle Database", path: "/vehicles", icon: "directions_car", min_role: Role::Enlisted },
            NavItem { label: "Modpacks", path: "/modpacks", icon: "extension", min_role: Role::Enlisted },
        ],
    },
    NavSection {
        title: "Administration",
        admin: true,
        items: &[
            NavItem { label: "Event Manager", path: "/admin/events", icon: "event_available", min_role: Role::Admin },
            NavItem { label: "Mission Approvals", path: "/admin/approvals", icon: "fact_check", min_role: Role::Admin },
            NavItem { label: "Server Control", path: "/admin/server", icon: "settings_system_daydream", min_role: Role::Admin },
            NavItem { label: "Personnel Roster", path: "/admin/personnel", icon: "groups", min_role: Role::Admin },
            NavItem { label: "Comms Broadcaster", path: "/admin/content", icon: "campaign", min_role: Role::Admin },
            NavItem { label: "Audit Logs", path: "/admin/audit", icon: "receipt_long", min_role: Role::Admin },
        ],
    },
];
