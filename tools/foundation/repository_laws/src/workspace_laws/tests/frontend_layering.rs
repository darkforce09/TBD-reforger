//! Tests for [`super`] — the layer order and the sub-area order over a fixture frontend crate.

use std::collections::BTreeSet;

use super::*;
use crate::temporary_checkout::TemporaryCheckout;

const ROWS: &[FrontendLayerRow] = &[
    FrontendLayerRow {
        path: "src/main.rs",
        layer: FrontendLayer::Shell,
        has_areas: false,
    },
    FrontendLayerRow {
        path: "src/shell",
        layer: FrontendLayer::Shell,
        has_areas: false,
    },
    FrontendLayerRow {
        path: "src/foundation",
        layer: FrontendLayer::Foundation,
        has_areas: false,
    },
    FrontendLayerRow {
        path: "src/pages",
        layer: FrontendLayer::Pages,
        has_areas: true,
    },
    FrontendLayerRow {
        path: "src/workspaces",
        layer: FrontendLayer::Workspaces,
        has_areas: true,
    },
];
const ORDERS: &[SubAreaOrder] = &[SubAreaOrder {
    parent: "src/foundation",
    tiers: &[&["ui"], &["transport"], &["auth"], &["offline", "map_view"]],
    test_only: &["test_support"],
}];
const CRATES: &[FrontendCrateLayers] = &[FrontendCrateLayers {
    crate_path: "apps/web",
    rows: ROWS,
    sub_area_orders: ORDERS,
}];

fn checkout(name: &str) -> TemporaryCheckout {
    let checkout = TemporaryCheckout::empty(&format!("layering-{name}"));
    for (rel, body) in [
        (
            "apps/web/src/main.rs",
            "mod foundation;\nmod pages;\nmod shell;\nmod workspaces;\nuse crate::pages::account::Page;\n",
        ),
        (
            "apps/web/src/shell/frame.rs",
            "use crate::foundation::auth::Role;\n",
        ),
        (
            "apps/web/src/foundation/mod.rs",
            "pub mod auth;\npub mod ui;\n#[cfg(test)]\npub mod test_support;\n",
        ),
        (
            "apps/web/src/foundation/ui/select.rs",
            "use super::tokens::Spacing;\n",
        ),
        (
            "apps/web/src/foundation/auth/store.rs",
            "use crate::foundation::{transport::Client, ui::select::Select};\n",
        ),
        (
            "apps/web/src/pages/account/page.rs",
            "use crate::foundation::ui::select::Select;\n",
        ),
        (
            "apps/web/src/pages/mission_hub/review.rs",
            "use crate::pages::mission_hub::library;\n",
        ),
        (
            "apps/web/src/workspaces/editor/mod.rs",
            "use crate::foundation::ui;\n",
        ),
    ] {
        checkout.write(rel, body);
    }
    checkout
}

fn scan(checkout: &TemporaryCheckout) -> LayeringScan {
    layering_edges(checkout.root(), &CRATES[0]).expect("the scan runs")
}

fn edges(checkout: &TemporaryCheckout) -> Vec<LayeringEdge> {
    let scan = scan(checkout);
    assert_eq!(scan.unmapped, Vec::<String>::new());
    assert_eq!(scan.unordered, BTreeSet::new());
    scan.edges
}

/// The (from, to, test) triples of `edges`, sorted.
fn triples(edges: Vec<LayeringEdge>) -> Vec<(String, String, bool)> {
    let mut found: Vec<(String, String, bool)> = edges
        .into_iter()
        .map(|edge| (edge.from, edge.to, edge.test))
        .collect();
    found.sort();
    found
}

fn owned(expected: &[(&str, &str, bool)]) -> Vec<(String, String, bool)> {
    expected
        .iter()
        .map(|(from, to, test)| (from.to_string(), to.to_string(), *test))
        .collect()
}

#[test]
fn frontend_layering_a_tree_in_order_passes() {
    let checkout = checkout("green");
    let report = check_frontend_layering(checkout.root(), CRATES);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines.first().map(String::as_str),
        Some(
            "==> frontend-layering — 0 production and 0 test layering edge(s); the law allows none"
        )
    );
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("FRONTEND-LAYERING: PASS")
    );
}

#[test]
fn frontend_layering_a_single_foundation_import_of_a_workspace_fails() {
    let checkout = checkout("foundation-workspace");
    checkout.write(
        "apps/web/src/foundation/ui/slider.rs",
        "use crate::{\n    workspaces::editor::tokens::Track,\n    foundation::ui,\n};\nfn f() { crate::workspaces::editor::purge(); }\n",
    );
    let found = edges(&checkout);
    assert_eq!(
        found.len(),
        1,
        "one edge however many lines name it: {found:#?}"
    );
    assert_eq!(
        (
            found[0].from.as_str(),
            found[0].to.as_str(),
            found[0].line_no
        ),
        ("foundation/ui", "workspaces/editor", 1)
    );
    let report = check_frontend_layering(checkout.root(), CRATES);
    assert_eq!(report.exit_code, 1);
    assert!(
        report.lines.contains(
            &"FAIL: production edge apps/web/src/foundation/ui/slider.rs:1: foundation/ui imports \
          workspaces/editor"
                .to_string()
        )
    );
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("FRONTEND-LAYERING: FAIL (1 finding(s))")
    );
}

#[test]
fn frontend_layering_pages_workspaces_shell_and_page_areas_are_judged() {
    let checkout = checkout("rules");
    checkout.write(
        "apps/web/src/pages/mission_hub/upload.rs",
        "use crate::workspaces::editor::Draft;\n",
    );
    checkout.write(
        "apps/web/src/workspaces/editor/review.rs",
        "use crate::pages::mission_hub::Record;\n",
    );
    checkout.write(
        "apps/web/src/pages/administration/approvals.rs",
        "use crate::pages::mission_hub::review::Thread;\n",
    );
    checkout.write(
        "apps/web/src/foundation/auth/tests/guard.rs",
        "fn t() { assert!(crate::shell::frame::allowed()); }\n",
    );
    assert_eq!(
        triples(edges(&checkout)),
        owned(&[
            ("foundation/auth", "shell", true),
            ("pages/administration", "pages/mission_hub", false),
            ("pages/mission_hub", "workspaces/editor", false),
            ("workspaces/editor", "pages/mission_hub", false),
        ])
    );
    let report = check_frontend_layering(checkout.root(), CRATES);
    assert_eq!(report.exit_code, 1);
    assert_eq!(
        report.lines.first().map(String::as_str),
        Some(
            "==> frontend-layering — 3 production and 1 test layering edge(s); the law allows none"
        )
    );
    assert!(
        report.lines.contains(
            &"FAIL: test edge apps/web/src/foundation/auth/tests/guard.rs:1: foundation/auth \
          imports shell"
                .to_string()
        )
    );
}

#[test]
fn frontend_layering_comments_strings_and_relative_paths_resolve_correctly() {
    let checkout = checkout("lexing");
    checkout.write(
        "apps/web/src/foundation/ui/text.rs",
        "//! See [`crate::workspaces::editor`].\n/* crate::shell */\nconst S: &str = \"crate::workspaces::editor\";\nconst R: &str = r#\"crate::foundation::auth\"#;\nmacro_rules! m { () => { $crate::workspaces::x() } }\n",
    );
    checkout.write(
        "apps/web/src/foundation/mod.rs",
        "pub mod auth;\nuse super::workspaces::editor::Tool;\n",
    );
    let found = edges(&checkout);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        (found[0].file.as_str(), found[0].to.as_str()),
        ("apps/web/src/foundation/mod.rs", "workspaces/editor")
    );
}

#[test]
fn frontend_layering_an_unmapped_file_is_a_finding_and_a_missing_crate_did_not_run() {
    let checkout = checkout("unmapped");
    checkout.write("apps/web/src/stray.rs", "fn f() {}\n");
    let report = check_frontend_layering(checkout.root(), CRATES);
    assert_eq!(report.exit_code, 1);
    assert!(
        report.lines.contains(
            &"FAIL: apps/web/src/stray.rs sits under no row of the layer table".to_string()
        )
    );
    std::fs::remove_dir_all(checkout.root().join("apps/web/src")).unwrap();
    assert_eq!(
        check_frontend_layering(checkout.root(), CRATES).exit_code,
        2
    );
}

#[test]
fn frontend_layering_sub_area_downward_imports_pass_and_upward_imports_fail() {
    let checkout = checkout("sub-area-order");
    checkout.write(
        "apps/web/src/foundation/offline/pack.rs",
        "use crate::foundation::auth::Role;\nuse super::super::transport::Client;\nuse crate::foundation::ui::Button;\nuse super::cache::Store;\n",
    );
    assert_eq!(
        edges(&checkout),
        Vec::new(),
        "downward and own-area imports"
    );
    checkout.write(
        "apps/web/src/foundation/transport/client.rs",
        "use super::headers;\nfn f() { super::super::auth::store::token(); }\n",
    );
    checkout.write(
        "apps/web/src/foundation/ui/tests/select.rs",
        "use crate::foundation::transport::Client;\n",
    );
    assert_eq!(
        triples(edges(&checkout)),
        owned(&[
            ("foundation/transport", "foundation/auth", false),
            ("foundation/ui", "foundation/transport", true),
        ])
    );
}

#[test]
fn frontend_layering_peer_sub_areas_never_import_each_other() {
    let checkout = checkout("peers");
    checkout.write(
        "apps/web/src/foundation/offline/pack.rs",
        "use crate::foundation::map_view::Mount;\n",
    );
    checkout.write(
        "apps/web/src/foundation/map_view/mount.rs",
        "use crate::foundation::offline::pack::Pack;\n",
    );
    assert_eq!(
        triples(edges(&checkout)),
        owned(&[
            ("foundation/map_view", "foundation/offline", false),
            ("foundation/offline", "foundation/map_view", false),
        ])
    );
}

#[test]
fn frontend_layering_only_test_files_import_a_test_only_sub_area() {
    let checkout = checkout("test-only");
    checkout.write(
        "apps/web/src/foundation/test_support/fixtures.rs",
        "use super::pins::source;\nuse crate::foundation::{auth::Role, offline::Pack};\n",
    );
    checkout.write(
        "apps/web/src/pages/account/tests/page.rs",
        "use crate::foundation::test_support::fixtures::golden;\n",
    );
    checkout.write(
        "apps/web/src/foundation/auth/store_tests.rs",
        "use crate::foundation::test_support::fixtures::golden;\n",
    );
    assert_eq!(
        edges(&checkout),
        Vec::new(),
        "test files and the area itself"
    );
    checkout.write(
        "apps/web/src/pages/account/form.rs",
        "fn f() { crate::foundation::test_support::fixtures::golden(); }\n",
    );
    checkout.write(
        "apps/web/src/foundation/ui/select.rs",
        "use crate::foundation::test_support::pins;\n",
    );
    assert_eq!(
        triples(edges(&checkout)),
        owned(&[
            ("foundation/ui", "foundation/test_support", false),
            ("pages/account", "foundation/test_support", false),
        ])
    );
}

#[test]
fn frontend_layering_a_grouped_use_across_two_sub_areas_is_judged_per_item() {
    let checkout = checkout("grouped");
    checkout.write(
        "apps/web/src/foundation/transport/client.rs",
        "use crate::foundation::{\n    ui::select::Select,\n    auth::{Role, store::Token},\n};\n",
    );
    checkout.write(
        "apps/web/src/foundation/ui/button.rs",
        "use super::super::{ui::select::Select, transport::Client, offline::Pack};\n",
    );
    let found = edges(&checkout);
    assert_eq!(
        triples(found.clone()),
        owned(&[
            ("foundation/transport", "foundation/auth", false),
            ("foundation/ui", "foundation/offline", false),
            ("foundation/ui", "foundation/transport", false),
        ])
    );
    let auth = found
        .iter()
        .find(|edge| edge.to == "foundation/auth")
        .expect("the auth edge");
    assert_eq!(
        auth.line_no, 1,
        "a grouped use reports the line it starts on"
    );
}

#[test]
fn frontend_layering_a_child_of_an_ordered_folder_in_no_tier_is_a_finding() {
    let checkout = checkout("unordered");
    checkout.write("apps/web/src/foundation/stray/mod.rs", "fn f() {}\n");
    checkout.write("apps/web/src/foundation/loose.rs", "fn f() {}\n");
    assert_eq!(
        scan(&checkout).unordered,
        BTreeSet::from([
            "apps/web/src/foundation/loose".to_string(),
            "apps/web/src/foundation/stray".to_string(),
        ])
    );
    let report = check_frontend_layering(checkout.root(), CRATES);
    assert_eq!(report.exit_code, 1);
    assert!(report.lines.contains(
        &"FAIL: apps/web/src/foundation/stray sits in no tier of its folder's order".to_string()
    ));
}
