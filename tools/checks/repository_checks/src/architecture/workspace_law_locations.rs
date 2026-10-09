//! The locations the workspace laws (`cargo xtask verify crate-tiers` and its siblings) read.
//!
//! **Role:** the application packages no member may depend on, the stylesheet whose `@source` lines must name every leptos crate, and
//! the frontend-layering configuration: the in-crate layer table of the app, its module orders,
//! and the layer folders and crate orders of the frontend crates.
//! **Position:** read by [`super::workspace_laws`]; the laws themselves live in
//! `repository_laws::workspace_laws` and know no path or crate name that moves with the tree, so
//! a stage that moves a folder or births a crate rewrites these constants.
//! **Signals & state:** none; constants.
//! **Invariants:** every path is repository-relative or (in a layer table) crate-relative; a crate
//! order and the application list name packages, each the name of its folder and each a workspace
//! member (an absent one is a finding of its law).

use repository_laws::workspace_laws::crate_tiers::CrateTierConfiguration;
use repository_laws::workspace_laws::frontend_layering::crate_edges::{
    FrontendCrateEdges, FrontendLayerFolder,
};
use repository_laws::workspace_laws::frontend_layering::{
    FrontendCrateLayers, FrontendLayer, FrontendLayerRow, FrontendLayering, SubAreaOrder,
    SubAreaTier,
};

/// The application packages: the binaries the platform deploys or runs — the API server, the
/// single-page app and its offline service worker, the game server host agent and the ticketboard
/// desktop viewer. No member depends on one, in any table (the crate-tier law).
pub(crate) const APPLICATION_PACKAGES: &[&str] = &[
    "api_server",
    "frontend_application",
    "offline_service_worker",
    "game_server_host_agent",
    "ticketboard_desktop",
];

/// What the crate-tier law reads: the application packages.
pub(crate) const CRATE_TIERS: &CrateTierConfiguration<'static> = &CrateTierConfiguration {
    application_packages: APPLICATION_PACKAGES,
};

/// The app stylesheet whose `@source` lines must name every leptos crate.
pub(crate) const TAILWIND_STYLESHEET: &str =
    "crates/frontend/shell/frontend_application/style/aegis.css";

/// The frontend-layering configuration: the in-crate mode over the app's own modules and the
/// crate-edge mode over the frontend crates.
pub(crate) const FRONTEND_LAYERS: &FrontendLayering = &FrontendLayering {
    in_crate: &[APP_LAYERS],
    crate_edges: FRONTEND_CRATE_EDGES,
};

/// The app's in-crate layer table: the entry point, the render form of the route table, the
/// platform frame and the crate-level tests are all the shell. The foundation, the features, the
/// pages and the workspaces are crates under `crates/frontend/`, which the crate-edge mode judges.
pub(crate) const APP_LAYERS: FrontendCrateLayers = FrontendCrateLayers {
    crate_path: "crates/frontend/shell/frontend_application",
    rows: &[
        shell("src/main.rs"),
        shell("src/app_routes.rs"),
        shell("src/tests"),
        shell("src/shell"),
    ],
    sub_area_orders: &[],
};

/// The crate-edge mode: every crate under `crates/frontend/<layer>/` has the layer of its folder,
/// the app `crates/frontend/shell/frontend_application` and the offline service worker beside it
/// in `crates/frontend/shell` being the shell; the foundation, workspace and shell crates follow
/// their crate orders, and every crate an order names must be a workspace member.
pub(crate) const FRONTEND_CRATE_EDGES: FrontendCrateEdges = FrontendCrateEdges {
    shell_crate: "crates/frontend/shell/frontend_application",
    crates_root: "crates/frontend",
    layer_folders: &[
        layer_folder("crates/frontend/foundation", FrontendLayer::Foundation),
        layer_folder("crates/frontend/features", FrontendLayer::Features),
        layer_folder("crates/frontend/pages", FrontendLayer::Pages),
        layer_folder("crates/frontend/workspaces", FrontendLayer::Workspaces),
        layer_folder("crates/frontend/shell", FrontendLayer::Shell),
    ],
    crate_orders: &[
        FOUNDATION_CRATE_ORDER,
        MISSION_CREATOR_CRATE_ORDER,
        DEBUG_BENCHES_CRATE_ORDER,
        SHELL_CRATE_ORDER,
    ],
};

/// The foundation crates, lowest first: `frontend_ui` < `frontend_api_dtos` <
/// {`frontend_transport`, `frontend_route_table`} < `frontend_session` < {`frontend_offline`,
/// `frontend_map_view`}, the braced tiers peers that never depend on each other;
/// `frontend_test_support` is reached only through dev-dependencies.
pub(crate) const FOUNDATION_CRATE_ORDER: SubAreaOrder = SubAreaOrder {
    parent: "crates/frontend/foundation",
    tiers: &[
        SubAreaTier::peers(&["frontend_ui"]),
        SubAreaTier::peers(&["frontend_api_dtos"]),
        SubAreaTier::peers(&["frontend_transport", "frontend_route_table"]),
        SubAreaTier::peers(&["frontend_session"]),
        SubAreaTier::peers(&["frontend_offline", "frontend_map_view"]),
    ],
    test_only: &["frontend_test_support"],
};

/// The Mission Creator's crates, lowest first: `mission_creator_state` <
/// `mission_creator_engine_bridge` < `mission_creator_session` < `mission_creator_arsenal` <
/// `mission_creator_workspace`.
pub(crate) const MISSION_CREATOR_CRATE_ORDER: SubAreaOrder = SubAreaOrder {
    parent: "crates/frontend/workspaces",
    tiers: &[
        SubAreaTier::peers(&["mission_creator_state"]),
        SubAreaTier::peers(&["mission_creator_engine_bridge"]),
        SubAreaTier::peers(&["mission_creator_session"]),
        SubAreaTier::peers(&["mission_creator_arsenal"]),
        SubAreaTier::peers(&["mission_creator_workspace"]),
    ],
    test_only: &[],
};

/// The debug benches, an order of their own in the workspaces folder: they depend on no
/// Mission Creator crate, and no Mission Creator crate depends on them.
pub(crate) const DEBUG_BENCHES_CRATE_ORDER: SubAreaOrder = SubAreaOrder {
    parent: "crates/frontend/workspaces",
    tiers: &[SubAreaTier::peers(&["debug_benches"])],
    test_only: &[],
};

/// The shell crates, one tier of peers: the single-page app and the offline service worker are
/// two binaries Trunk builds side by side, so neither names the other, in any table.
pub(crate) const SHELL_CRATE_ORDER: SubAreaOrder = SubAreaOrder {
    parent: "crates/frontend/shell",
    tiers: &[SubAreaTier::peers(&[
        "frontend_application",
        "offline_service_worker",
    ])],
    test_only: &[],
};

const fn shell(path: &'static str) -> FrontendLayerRow {
    row(path, FrontendLayer::Shell, false)
}

const fn row(path: &'static str, layer: FrontendLayer, has_areas: bool) -> FrontendLayerRow {
    FrontendLayerRow {
        path,
        layer,
        has_areas,
    }
}

const fn layer_folder(path: &'static str, layer: FrontendLayer) -> FrontendLayerFolder {
    FrontendLayerFolder { path, layer }
}
