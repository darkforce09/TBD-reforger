//! The locations the workspace laws (`cargo xtask verify crate-tiers` and its siblings) read.
//!
//! **Role:** the folders whose manifests must be workspace members, the stylesheet whose `@source`
//! lines must name every leptos crate, and the frontend-layering configuration: the in-crate
//! layer table of the app, its module orders, and the layer folders and crate orders of the
//! frontend crates.
//! **Position:** read by [`super::workspace_laws`]; the laws themselves live in
//! `repository_laws::workspace_laws` and know no path or crate name that moves with the tree, so
//! a stage that moves a folder or births a crate rewrites these constants.
//! **Signals & state:** none; constants.
//! **Invariants:** every path is repository-relative or (in a layer table) crate-relative; a
//! sweep root that does not exist yet holds no manifest; a crate order names packages, each the
//! name of its folder.

use repository_laws::workspace_laws::frontend_layering::crate_edges::{
    FrontendCrateEdges, FrontendLayerFolder,
};
use repository_laws::workspace_laws::frontend_layering::{
    FrontendCrateLayers, FrontendLayer, FrontendLayerRow, FrontendLayering, SubAreaOrder,
    SubAreaTier,
};

/// Folders whose every `Cargo.toml` (outside test trees, fixtures and build output) must be a
/// workspace member. A folder that does not exist yet holds no manifest.
pub(crate) const MANIFEST_SWEEP_ROOTS: &[&str] = &["apps", "crates", "tools", "legacy"];

/// The app stylesheet whose `@source` lines must name every leptos crate.
pub(crate) const TAILWIND_STYLESHEET: &str = "apps/frontend/style/aegis.css";

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
    crate_path: "apps/frontend",
    rows: &[
        shell("src/main.rs"),
        shell("src/app_routes.rs"),
        shell("src/tests"),
        shell("src/shell"),
    ],
    sub_area_orders: &[],
};

/// The crate-edge mode: every crate under `crates/frontend/<layer>/` has the layer of its folder
/// and the app `apps/frontend` is the shell; the foundation and workspace crates follow their
/// crate orders, and every crate an order names must be a workspace member.
pub(crate) const FRONTEND_CRATE_EDGES: FrontendCrateEdges = FrontendCrateEdges {
    shell_crate: "apps/frontend",
    crates_root: "crates/frontend",
    layer_folders: &[
        layer_folder("crates/frontend/foundation", FrontendLayer::Foundation),
        layer_folder("crates/frontend/features", FrontendLayer::Features),
        layer_folder("crates/frontend/pages", FrontendLayer::Pages),
        layer_folder("crates/frontend/workspaces", FrontendLayer::Workspaces),
    ],
    crate_orders: &[
        FOUNDATION_CRATE_ORDER,
        MISSION_CREATOR_CRATE_ORDER,
        DEBUG_BENCHES_CRATE_ORDER,
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
