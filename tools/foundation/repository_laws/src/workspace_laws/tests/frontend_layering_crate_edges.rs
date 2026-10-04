//! Tests for [`super`] — the crate-edge mode of the frontend-layering law over fixture
//! workspaces: layer order, page peers, the foundation and workspace crate orders, test-only
//! crates, placement and absent configured crates.

use super::super::{FrontendLayering, SubAreaTier, check_frontend_layering};
use super::*;
use crate::workspace_laws::fixture_workspace::{
    Dependency, FixtureWorkspace, application_manifest,
};

const FOUNDATION: &str = "crates/frontend/foundation";
const WORKSPACES: &str = "crates/frontend/workspaces";
const NORMAL: &str = "dependencies";
const DEV: &str = "dev-dependencies";

const LAYER_FOLDERS: &[FrontendLayerFolder] = &[
    FrontendLayerFolder {
        path: FOUNDATION,
        layer: FrontendLayer::Foundation,
    },
    FrontendLayerFolder {
        path: "crates/frontend/features",
        layer: FrontendLayer::Features,
    },
    FrontendLayerFolder {
        path: "crates/frontend/pages",
        layer: FrontendLayer::Pages,
    },
    FrontendLayerFolder {
        path: WORKSPACES,
        layer: FrontendLayer::Workspaces,
    },
];

const FOUNDATION_ORDER: SubAreaOrder = SubAreaOrder {
    parent: FOUNDATION,
    tiers: &[
        SubAreaTier::peers(&["frontend_ui"]),
        SubAreaTier::peers(&["frontend_api_dtos"]),
        SubAreaTier::peers(&["frontend_transport", "frontend_route_table"]),
        SubAreaTier::peers(&["frontend_session"]),
    ],
    test_only: &["frontend_test_support"],
};
/// The foundation order with a top tier naming `frontend_offline`, which [`GREEN`] never places.
const FOUNDATION_ORDER_NAMING_OFFLINE: SubAreaOrder = SubAreaOrder {
    tiers: &[
        SubAreaTier::peers(&["frontend_ui"]),
        SubAreaTier::peers(&["frontend_api_dtos"]),
        SubAreaTier::peers(&["frontend_transport", "frontend_route_table"]),
        SubAreaTier::peers(&["frontend_session"]),
        SubAreaTier::peers(&["frontend_offline"]),
    ],
    ..FOUNDATION_ORDER
};
const MISSION_CREATOR_ORDER: SubAreaOrder = SubAreaOrder {
    parent: WORKSPACES,
    tiers: &[
        SubAreaTier::peers(&["mission_creator_state"]),
        SubAreaTier::peers(&["mission_creator_engine_bridge"]),
        SubAreaTier::peers(&["mission_creator_workspace"]),
    ],
    test_only: &[],
};
const BENCHES_ORDER: SubAreaOrder = SubAreaOrder {
    parent: WORKSPACES,
    tiers: &[SubAreaTier::peers(&["debug_benches"])],
    test_only: &[],
};

const CONFIG: FrontendCrateEdges = FrontendCrateEdges {
    shell_crate: "apps/web",
    crates_root: "crates/frontend",
    layer_folders: LAYER_FOLDERS,
    crate_orders: &[FOUNDATION_ORDER, MISSION_CREATOR_ORDER, BENCHES_ORDER],
};

/// [`CONFIG`] with the foundation order naming `frontend_offline`.
const CONFIG_NAMING_OFFLINE: FrontendCrateEdges = FrontendCrateEdges {
    crate_orders: &[
        FOUNDATION_ORDER_NAMING_OFFLINE,
        MISSION_CREATOR_ORDER,
        BENCHES_ORDER,
    ],
    ..CONFIG
};

/// The green crate graph: every crate with the edges it may have.
const GREEN: &[(&str, &[(&str, &str)])] = &[
    ("crates/frontend/foundation/frontend_ui", &[]),
    (
        "crates/frontend/foundation/frontend_api_dtos",
        &[("frontend_ui", NORMAL)],
    ),
    (
        "crates/frontend/foundation/frontend_transport",
        &[
            ("frontend_api_dtos", NORMAL),
            ("frontend_test_support", DEV),
        ],
    ),
    (
        "crates/frontend/foundation/frontend_route_table",
        &[("frontend_api_dtos", NORMAL)],
    ),
    (
        "crates/frontend/foundation/frontend_session",
        &[
            ("frontend_transport", NORMAL),
            ("frontend_route_table", NORMAL),
            ("frontend_ui", NORMAL),
        ],
    ),
    (
        "crates/frontend/foundation/frontend_test_support",
        &[("frontend_ui", NORMAL), ("frontend_session", NORMAL)],
    ),
    (
        "crates/frontend/features/mission_review_record",
        &[("frontend_session", NORMAL)],
    ),
    (
        "crates/frontend/pages/account_pages",
        &[
            ("frontend_session", NORMAL),
            ("mission_review_record", NORMAL),
            ("frontend_test_support", DEV),
        ],
    ),
    (
        "crates/frontend/pages/mission_hub_pages",
        &[("mission_review_record", NORMAL)],
    ),
    (
        "crates/frontend/workspaces/mission_creator_state",
        &[("frontend_ui", NORMAL)],
    ),
    (
        "crates/frontend/workspaces/mission_creator_engine_bridge",
        &[("mission_creator_state", NORMAL)],
    ),
    (
        "crates/frontend/workspaces/mission_creator_workspace",
        &[
            ("mission_creator_engine_bridge", NORMAL),
            ("mission_creator_state", NORMAL),
        ],
    ),
    (
        "crates/frontend/workspaces/debug_benches",
        &[("frontend_session", NORMAL)],
    ),
];

/// A fixture workspace of [`GREEN`] plus the `extra` (crate path, package, table) edges; the
/// app `apps/web` depends on every page and workspace crate.
fn workspace(name: &str, extra: &[(&str, &str, &str)]) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(&format!("crate-edges-{name}"));
    for (path, edges) in GREEN {
        let mut dependencies: Vec<Dependency<'_>> = edges
            .iter()
            .copied()
            .chain(
                extra
                    .iter()
                    .filter(|(at, _, _)| at == path)
                    .map(|&(_, package, table)| (package, table)),
            )
            .map(|(package, table)| Dependency { package, table })
            .collect();
        dependencies.sort_by_key(|dependency| dependency.table);
        workspace.layout_crate(path, 0, "any", &dependencies);
    }
    let mut app = String::new();
    for package in [
        "account_pages",
        "mission_hub_pages",
        "mission_creator_workspace",
        "debug_benches",
    ] {
        app.push_str(&format!("{package} = {{ workspace = true }}\n"));
    }
    for (_, package, table) in extra.iter().filter(|(at, _, _)| *at == "apps/web") {
        app.push_str(&format!(
            "\n[{table}]\n{package} = {{ workspace = true }}\n"
        ));
    }
    workspace.member("apps/web", &application_manifest("web", &app));
    workspace
}

fn scan(workspace: &FixtureWorkspace) -> CrateEdgeScan {
    crate_edge_scan(workspace.root(), &CONFIG).expect("the scan runs")
}

/// The (from, to, dev) triples of the scan's edges, sorted.
fn triples(workspace: &FixtureWorkspace) -> Vec<(String, String, bool)> {
    let scan = scan(workspace);
    assert_eq!(scan.misplaced, Vec::<String>::new());
    let mut found: Vec<(String, String, bool)> = scan
        .edges
        .into_iter()
        .map(|edge| (edge.from, edge.to, edge.test))
        .collect();
    found.sort();
    found
}

fn owned(expected: &[(&str, &str, bool)]) -> Vec<(String, String, bool)> {
    let mut owned: Vec<(String, String, bool)> = expected
        .iter()
        .map(|(from, to, dev)| (from.to_string(), to.to_string(), *dev))
        .collect();
    owned.sort();
    owned
}

fn layering(crate_edges: FrontendCrateEdges) -> FrontendLayering {
    FrontendLayering {
        in_crate: &[],
        crate_edges,
    }
}

/// The 1-based line of `package`'s key in the manifest of the crate at `path`.
fn line_of(workspace: &FixtureWorkspace, path: &str, package: &str) -> usize {
    let manifest = std::fs::read_to_string(workspace.root().join(path).join("Cargo.toml")).unwrap();
    manifest
        .lines()
        .position(|line| line.starts_with(&format!("{package} =")))
        .expect("the key")
        + 1
}

#[test]
fn frontend_crate_edges_a_graph_in_order_passes() {
    let workspace = workspace("green", &[]);
    let found = scan(&workspace);
    assert_eq!(found.edges, Vec::new());
    assert_eq!(found.misplaced, Vec::<String>::new());
    assert_eq!(found.absent, Vec::<String>::new());
    assert_eq!(found.crates.len(), GREEN.len() + 1, "{:?}", found.crates);
    let report = check_frontend_layering(workspace.root(), &layering(CONFIG));
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines,
        vec![
            format!(
                "==> frontend-layering — 0 production and 0 test layering edge(s) between \
                 modules; 0 normal and 0 dev edge(s) between {} frontend crate(s); the law \
                 allows none",
                GREEN.len() + 1
            ),
            "FRONTEND-LAYERING: PASS".to_string(),
        ]
    );
}

#[test]
fn frontend_crate_edges_a_crate_an_order_names_that_no_member_carries_is_a_finding() {
    let workspace = workspace("absent-finding", &[]);
    let found = crate_edge_scan(workspace.root(), &CONFIG_NAMING_OFFLINE).expect("the scan runs");
    assert_eq!(found.absent, vec!["frontend_offline".to_string()]);
    let report = check_frontend_layering(workspace.root(), &layering(CONFIG_NAMING_OFFLINE));
    assert_eq!(report.exit_code, 1);
    assert!(
        report.lines.contains(
            &"FAIL: frontend_offline is named in a frontend crate order but is no workspace member"
                .to_string()
        )
    );
}

#[test]
fn frontend_crate_edges_a_back_edge_onto_a_higher_layer_is_a_finding() {
    let workspace = workspace(
        "back-edge",
        &[
            (
                "crates/frontend/foundation/frontend_ui",
                "account_pages",
                NORMAL,
            ),
            (
                "crates/frontend/features/mission_review_record",
                "mission_creator_state",
                DEV,
            ),
            (
                "crates/frontend/pages/mission_hub_pages",
                "mission_creator_workspace",
                NORMAL,
            ),
            ("crates/frontend/workspaces/debug_benches", "web", DEV),
        ],
    );
    assert_eq!(
        triples(&workspace),
        owned(&[
            ("foundation/frontend_ui", "pages/account_pages", false),
            (
                "features/mission_review_record",
                "workspaces/mission_creator_state",
                true
            ),
            (
                "pages/mission_hub_pages",
                "workspaces/mission_creator_workspace",
                false
            ),
            ("workspaces/debug_benches", "shell", true),
        ])
    );
    let report = check_frontend_layering(workspace.root(), &layering(CONFIG));
    assert_eq!(report.exit_code, 1);
    let line = line_of(
        &workspace,
        "crates/frontend/foundation/frontend_ui",
        "account_pages",
    );
    assert!(
        report.lines.contains(&format!(
            "FAIL: normal crate edge crates/frontend/foundation/frontend_ui/Cargo.toml:{line}: \
             foundation/frontend_ui depends on pages/account_pages"
        )),
        "{}",
        report.lines.join("\n")
    );
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("FRONTEND-LAYERING: FAIL (4 finding(s))")
    );
}

#[test]
fn frontend_crate_edges_page_crates_never_depend_on_each_other() {
    let workspace = workspace(
        "peer-pages",
        &[
            (
                "crates/frontend/pages/account_pages",
                "mission_hub_pages",
                NORMAL,
            ),
            (
                "crates/frontend/pages/mission_hub_pages",
                "account_pages",
                DEV,
            ),
        ],
    );
    assert_eq!(
        triples(&workspace),
        owned(&[
            ("pages/account_pages", "pages/mission_hub_pages", false),
            ("pages/mission_hub_pages", "pages/account_pages", true),
        ])
    );
}

#[test]
fn frontend_crate_edges_the_foundation_order_and_its_test_only_crate_hold() {
    let workspace = workspace(
        "foundation-order",
        &[
            (
                "crates/frontend/foundation/frontend_api_dtos",
                "frontend_transport",
                NORMAL,
            ),
            (
                "crates/frontend/foundation/frontend_transport",
                "frontend_route_table",
                NORMAL,
            ),
            (
                "crates/frontend/foundation/frontend_ui",
                "frontend_session",
                DEV,
            ),
            (
                "crates/frontend/pages/mission_hub_pages",
                "frontend_test_support",
                NORMAL,
            ),
            ("apps/web", "frontend_test_support", NORMAL),
        ],
    );
    assert_eq!(
        triples(&workspace),
        owned(&[
            (
                "foundation/frontend_api_dtos",
                "foundation/frontend_transport",
                false
            ),
            (
                "foundation/frontend_transport",
                "foundation/frontend_route_table",
                false
            ),
            (
                "foundation/frontend_ui",
                "foundation/frontend_session",
                true
            ),
            (
                "pages/mission_hub_pages",
                "foundation/frontend_test_support",
                false
            ),
            ("shell", "foundation/frontend_test_support", false),
        ])
    );
}

#[test]
fn frontend_crate_edges_the_workspace_order_and_independent_orders_hold() {
    let workspace = workspace(
        "workspace-order",
        &[
            (
                "crates/frontend/workspaces/mission_creator_state",
                "mission_creator_engine_bridge",
                NORMAL,
            ),
            (
                "crates/frontend/workspaces/debug_benches",
                "mission_creator_state",
                NORMAL,
            ),
            (
                "crates/frontend/workspaces/mission_creator_workspace",
                "debug_benches",
                DEV,
            ),
        ],
    );
    assert_eq!(
        triples(&workspace),
        owned(&[
            (
                "workspaces/debug_benches",
                "workspaces/mission_creator_state",
                false
            ),
            (
                "workspaces/mission_creator_state",
                "workspaces/mission_creator_engine_bridge",
                false
            ),
            (
                "workspaces/mission_creator_workspace",
                "workspaces/debug_benches",
                true
            ),
        ])
    );
}

#[test]
fn frontend_crate_edges_an_edge_declared_twice_is_one_finding() {
    let workspace = workspace(
        "twice",
        &[
            (
                "crates/frontend/foundation/frontend_ui",
                "frontend_session",
                NORMAL,
            ),
            (
                "crates/frontend/foundation/frontend_ui",
                "frontend_session",
                "target.'cfg(target_arch = \"wasm32\")'.dependencies",
            ),
        ],
    );
    assert_eq!(
        triples(&workspace),
        owned(&[(
            "foundation/frontend_ui",
            "foundation/frontend_session",
            false
        )])
    );
}

#[test]
fn frontend_crate_edges_unknown_and_misplaced_crates_are_findings() {
    let mut workspace = workspace("placement", &[]);
    workspace.layout_crate("crates/frontend/foundation/frontend_stray", 0, "any", &[]);
    workspace.layout_crate("crates/frontend/widgets/widget_kit", 0, "any", &[]);
    workspace.layout_crate("crates/frontend/features/frontend_offline", 0, "any", &[]);
    workspace.layout_crate("crates/frontend/pages/event_pages", 0, "any", &[]);
    let found = crate_edge_scan(workspace.root(), &CONFIG_NAMING_OFFLINE).expect("the scan runs");
    assert_eq!(
        found.misplaced,
        vec![
            "crates/frontend/foundation/frontend_stray sits in no crate order of \
             crates/frontend/foundation"
                .to_string(),
            "crates/frontend/widgets/widget_kit sits in no frontend layer folder \
             (crates/frontend/foundation, crates/frontend/features, crates/frontend/pages, \
             crates/frontend/workspaces)"
                .to_string(),
            "crates/frontend/features/frontend_offline is named in the crate order of \
             crates/frontend/foundation but sits in crates/frontend/features"
                .to_string(),
        ]
    );
    assert_eq!(found.absent, Vec::<String>::new());
    let report = check_frontend_layering(workspace.root(), &layering(CONFIG_NAMING_OFFLINE));
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("FRONTEND-LAYERING: FAIL (3 finding(s))")
    );
}

#[test]
fn frontend_crate_edges_a_missing_app_or_root_manifest_did_not_run() {
    let workspace = workspace("missing-app", &[]);
    let elsewhere = FrontendCrateEdges {
        shell_crate: "apps/elsewhere",
        ..CONFIG
    };
    assert_eq!(
        check_frontend_layering(workspace.root(), &layering(elsewhere)).exit_code,
        2
    );
    std::fs::remove_file(workspace.root().join("Cargo.toml")).unwrap();
    assert_eq!(
        check_frontend_layering(workspace.root(), &layering(CONFIG)).exit_code,
        2
    );
}
