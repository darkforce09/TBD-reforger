//! The locations the workspace laws (`cargo xtask verify crate-tiers` and its siblings) read.
//!
//! **Role:** the folders whose manifests must be workspace members, the stylesheet whose `@source`
//! lines must cover every leptos crate, and the frontend crate's layer table.
//! **Position:** read by [`super::workspace_laws`]; the laws themselves live in
//! `repository_laws::workspace_laws` and know no path that moves with the tree, so a stage that
//! moves a folder rewrites these constants.
//! **Signals & state:** none; constants.
//! **Invariants:** every path is repository-relative; a sweep root that does not exist yet holds
//! no manifest.

use repository_laws::workspace_laws::frontend_layering::{
    FrontendCrateLayers, FrontendLayer, FrontendLayerRow, SubAreaOrder,
};

/// Folders whose every `Cargo.toml` (outside test trees, fixtures and build output) must be a
/// workspace member. A folder that does not exist yet holds no manifest.
pub(crate) const MANIFEST_SWEEP_ROOTS: &[&str] = &["apps", "crates", "tools", "legacy"];

/// The app stylesheet whose `@source` lines must cover every leptos crate.
pub(crate) const TAILWIND_STYLESHEET: &str = "apps/frontend/style/aegis.css";

/// The frontend crate's layer table: `src/foundation` is the foundation, `src/features` the
/// shared features, `src/pages` the pages (one area per section), `src/workspaces` the
/// workspaces (one area per workspace), and the entry point, the render form of the route
/// table, the platform frame and the crate-level tests the shell. Inside the foundation the
/// sub-areas follow [`FOUNDATION_SUB_AREA_ORDER`].
pub(crate) const FRONTEND_LAYERS: &[FrontendCrateLayers] = &[FrontendCrateLayers {
    crate_path: "apps/frontend",
    rows: &[
        shell("src/main.rs"),
        shell("src/app_routes.rs"),
        shell("src/tests"),
        shell("src/shell"),
        row("src/foundation", FrontendLayer::Foundation, false),
        row("src/features", FrontendLayer::Features, false),
        row("src/pages", FrontendLayer::Pages, true),
        row("src/workspaces", FrontendLayer::Workspaces, true),
    ],
    sub_area_orders: &[FOUNDATION_SUB_AREA_ORDER],
}];

/// The strict order of the foundation's sub-areas, lowest first: `ui` < `utils` <
/// `transport` < `route_table` < `auth` < {`offline`, `map_view`}. A sub-area imports only
/// sub-areas strictly before it; `offline` and `map_view` are peers that never import each
/// other; `test_support` sits outside the order and only test files import it.
pub(crate) const FOUNDATION_SUB_AREA_ORDER: SubAreaOrder = SubAreaOrder {
    parent: "src/foundation",
    tiers: &[
        &["ui"],
        &["utils"],
        &["transport"],
        &["route_table"],
        &["auth"],
        &["offline", "map_view"],
    ],
    test_only: &["test_support"],
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
