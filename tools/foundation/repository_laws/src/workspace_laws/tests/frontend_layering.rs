//! Tests for [`super`] — the layer order over a fixture frontend crate.

use super::*;
use crate::temporary_checkout::TemporaryCheckout;

const ROWS: &[FrontendLayerRow] = &[
    FrontendLayerRow {
        path: "src/main.rs",
        layer: FrontendLayer::Shell,
        has_areas: false,
    },
    FrontendLayerRow {
        path: "src/router.rs",
        layer: FrontendLayer::Shell,
        has_areas: false,
    },
    FrontendLayerRow {
        path: "src/v2/mod.rs",
        layer: FrontendLayer::Shell,
        has_areas: false,
    },
    FrontendLayerRow {
        path: "src/v2/core",
        layer: FrontendLayer::Foundation,
        has_areas: false,
    },
    FrontendLayerRow {
        path: "src/v2/pages",
        layer: FrontendLayer::Pages,
        has_areas: true,
    },
    FrontendLayerRow {
        path: "src/v2/apps",
        layer: FrontendLayer::Workspaces,
        has_areas: true,
    },
];
const CRATES: &[FrontendCrateLayers] = &[FrontendCrateLayers {
    crate_path: "apps/web",
    rows: ROWS,
}];
const ZERO: LayeringCeiling = LayeringCeiling {
    production: 0,
    test: 0,
};

fn checkout(name: &str) -> TemporaryCheckout {
    let checkout = TemporaryCheckout::empty(&format!("layering-{name}"));
    for (rel, body) in [
        (
            "apps/web/src/main.rs",
            "mod router;\nmod v2;\nuse crate::v2::pages::account::Page;\n",
        ),
        (
            "apps/web/src/router.rs",
            "use crate::v2::core::auth::Role;\n",
        ),
        (
            "apps/web/src/v2/mod.rs",
            "pub mod apps;\npub mod core;\npub mod pages;\n",
        ),
        (
            "apps/web/src/v2/core/ui/select.rs",
            "use super::tokens::Spacing;\n",
        ),
        (
            "apps/web/src/v2/pages/account/page.rs",
            "use crate::v2::core::ui::select::Select;\n",
        ),
        (
            "apps/web/src/v2/pages/mission_hub/review.rs",
            "use crate::v2::pages::mission_hub::library;\n",
        ),
        (
            "apps/web/src/v2/apps/editor/mod.rs",
            "use crate::v2::core::ui;\n",
        ),
    ] {
        checkout.write(rel, body);
    }
    checkout
}

fn edges(checkout: &TemporaryCheckout) -> Vec<LayeringEdge> {
    let (edges, unmapped) = layering_edges(checkout.root(), &CRATES[0]).expect("the scan runs");
    assert_eq!(unmapped, Vec::<String>::new());
    edges
}

#[test]
fn frontend_layering_a_tree_in_order_passes_at_a_zero_ceiling() {
    let checkout = checkout("green");
    let report = check_frontend_layering(checkout.root(), CRATES, ZERO);
    assert_eq!(report.exit_code, 0, "{}", report.lines.join("\n"));
    assert_eq!(
        report.lines.last().map(String::as_str),
        Some("FRONTEND-LAYERING: PASS")
    );
}

#[test]
fn frontend_layering_a_foundation_import_of_a_workspace_exceeds_the_ceiling() {
    let checkout = checkout("foundation-workspace");
    checkout.write(
        "apps/web/src/v2/core/ui/slider.rs",
        "use crate::v2::{\n    apps::editor::tokens::Track,\n    core::ui,\n};\nfn f() { crate::v2::apps::editor::purge(); }\n",
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
        ("foundation", "workspaces/editor", 1)
    );
    let report = check_frontend_layering(checkout.root(), CRATES, ZERO);
    assert_eq!(report.exit_code, 1);
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("1 production layering edge(s) exceed the ceiling of 0"))
    );
    let at_ceiling = LayeringCeiling {
        production: 1,
        test: 0,
    };
    assert_eq!(
        check_frontend_layering(checkout.root(), CRATES, at_ceiling).exit_code,
        0
    );
    let above = LayeringCeiling {
        production: 2,
        test: 0,
    };
    let report = check_frontend_layering(checkout.root(), CRATES, above);
    assert_eq!(report.exit_code, 0);
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("lower FRONTEND_LAYERING_CEILING.production to 1"))
    );
}

#[test]
fn frontend_layering_pages_workspaces_shell_and_page_areas_are_judged() {
    let checkout = checkout("rules");
    checkout.write(
        "apps/web/src/v2/pages/mission_hub/upload.rs",
        "use crate::v2::apps::editor::Draft;\n",
    );
    checkout.write(
        "apps/web/src/v2/apps/editor/review.rs",
        "use crate::v2::pages::mission_hub::Record;\n",
    );
    checkout.write(
        "apps/web/src/v2/pages/administration/approvals.rs",
        "use crate::v2::pages::mission_hub::review::Thread;\n",
    );
    checkout.write(
        "apps/web/src/v2/core/auth/tests/guard.rs",
        "fn t() { assert!(crate::router::allowed()); }\n",
    );
    let found: Vec<(String, String, bool)> = edges(&checkout)
        .into_iter()
        .map(|e| (e.from, e.to, e.test))
        .collect();
    let expected = [
        ("foundation", "shell", true),
        ("pages/administration", "pages/mission_hub", false),
        ("pages/mission_hub", "workspaces/editor", false),
        ("workspaces/editor", "pages/mission_hub", false),
    ];
    let expected: Vec<(String, String, bool)> = expected
        .iter()
        .map(|(f, t, test)| (f.to_string(), t.to_string(), *test))
        .collect();
    let mut found_sorted = found.clone();
    found_sorted.sort();
    assert_eq!(found_sorted, expected);
    let report = check_frontend_layering(
        checkout.root(),
        CRATES,
        LayeringCeiling {
            production: 3,
            test: 0,
        },
    );
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("1 test layering edge(s) exceed the ceiling of 0"))
    );
}

#[test]
fn frontend_layering_comments_strings_and_relative_paths_resolve_correctly() {
    let checkout = checkout("lexing");
    checkout.write(
        "apps/web/src/v2/core/ui/text.rs",
        "//! See [`crate::v2::apps::editor`].\n/* crate::router */\nconst S: &str = \"crate::v2::apps::editor\";\nconst R: &str = r#\"crate::router\"#;\nmacro_rules! m { () => { $crate::v2::apps::x() } }\n",
    );
    checkout.write(
        "apps/web/src/v2/core/mod.rs",
        "use super::apps::editor::Tool;\n",
    );
    let found = edges(&checkout);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        (found[0].file.as_str(), found[0].to.as_str()),
        ("apps/web/src/v2/core/mod.rs", "workspaces/editor")
    );
}

#[test]
fn frontend_layering_an_unmapped_file_is_a_finding_and_a_missing_crate_did_not_run() {
    let checkout = checkout("unmapped");
    checkout.write("apps/web/src/stray.rs", "fn f() {}\n");
    let report = check_frontend_layering(checkout.root(), CRATES, ZERO);
    assert_eq!(report.exit_code, 1);
    assert!(
        report.lines.contains(
            &"FAIL: apps/web/src/stray.rs sits under no row of the layer table".to_string()
        )
    );
    std::fs::remove_dir_all(checkout.root().join("apps/web/src")).unwrap();
    assert_eq!(
        check_frontend_layering(checkout.root(), CRATES, ZERO).exit_code,
        2
    );
}
