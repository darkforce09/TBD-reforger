use super::*;
use crate::cli::{Cli, TopCmd};
use crate::commands::verify::cli::VerifyCmd;
use clap::Parser;

fn tree() -> TrackedTree {
    TrackedTree::from_listing("apps/a/b/c.rs\0apps/a/README.md\0apps/bc/d.rs\0tools_v2/x.rs")
}

fn resolve(values: &[&str]) -> Result<GateScope, ScopeRefusal> {
    let values: Vec<String> = values.iter().map(ToString::to_string).collect();
    GateScope::resolve(&values, Path::new("/checkout"), &tree())
}

#[test]
fn no_value_and_the_root_spellings_mean_the_whole_repository() {
    let cases: [&[&str]; 5] = [&[], &["."], &["./"], &["/checkout"], &["apps/a", "."]];
    for values in cases {
        let scope = resolve(values).expect("a whole-repository scope");
        assert_eq!(scope.describe(), "the whole repository", "{values:?}");
        assert!(scope.contains("tools_v2/x.rs"));
        assert!(scope.overlaps("anything"));
        assert_eq!(scope.anchor(Path::new("/checkout")), Path::new("/checkout"));
    }
}

#[test]
fn a_folder_value_is_normalised_and_selects_its_subtree_only() {
    let scope = resolve(&["./apps//a/"]).expect("a tracked folder");
    assert_eq!(scope.describe(), "apps/a");
    assert!(scope.contains("apps/a"));
    assert!(scope.contains("apps/a/b/c.rs"));
    assert!(!scope.contains("apps"));
    assert!(!scope.contains("apps/ab/x.rs"));
    assert_eq!(
        scope.anchor(Path::new("/checkout")),
        Path::new("/checkout/apps/a")
    );
}

#[test]
fn an_absolute_value_inside_the_checkout_is_relative_to_it() {
    let scope = resolve(&["/checkout/apps/bc"]).expect("inside the checkout");
    assert_eq!(scope.describe(), "apps/bc");
}

#[test]
fn repeated_values_add_folders_once_each() {
    let scope = resolve(&["apps/a", "tools_v2", "apps/a/"]).expect("tracked folders");
    assert_eq!(scope.describe(), "apps/a, tools_v2");
    assert!(scope.contains("tools_v2/x.rs"));
    assert!(!scope.contains("apps/bc/d.rs"));
}

#[test]
fn a_scope_overlaps_the_folders_around_and_inside_it() {
    let scope = resolve(&["apps/a"]).expect("a tracked folder");
    assert!(scope.overlaps("apps"), "apps holds the scope folder");
    assert!(scope.overlaps("apps/a/b"), "the scope holds apps/a/b");
    assert!(!scope.overlaps("apps/bc"));
    assert!(!scope.overlaps("tools_v2"));
}

#[test]
fn a_value_that_names_no_tracked_folder_is_refused() {
    let cases = [
        ("apps/missing", "names no tracked folder"),
        ("apps/a/b/c.rs", "is a file; --path takes a folder"),
        ("apps/../tools_v2", "climbs out of its folder with `..`"),
        ("/elsewhere/apps", "lies outside the checkout"),
    ];
    for (value, reason) in cases {
        let refusal = resolve(&[value]).expect_err(value);
        assert_eq!(
            refusal,
            ScopeRefusal {
                value: value.to_string(),
                reason
            }
        );
    }
}

#[test]
fn a_bad_value_is_refused_even_beside_a_whole_repository_value() {
    let refusal = resolve(&[".", "apps/missing"]).expect_err("the typo must be refused");
    assert_eq!(refusal.value, "apps/missing");
}

#[test]
fn both_verbs_take_a_repeatable_path() {
    let parse = |verb: &str| {
        Cli::try_parse_from([
            "xtask", "verify", verb, "--path", "apps", "--path", "tools_v2",
        ])
        .expect("the verb parses")
    };
    match parse("readme-coverage").cmd {
        TopCmd::Verify {
            cmd: VerifyCmd::ReadmeCoverage { paths },
        } => assert_eq!(paths, ["apps", "tools_v2"]),
        other => panic!("readme-coverage parsed as {other:?}"),
    }
    match parse("markdown-placement").cmd {
        TopCmd::Verify {
            cmd: VerifyCmd::MarkdownPlacement { paths },
        } => assert_eq!(paths, ["apps", "tools_v2"]),
        other => panic!("markdown-placement parsed as {other:?}"),
    }
    let bare = Cli::try_parse_from(["xtask", "verify", "readme-coverage"]).expect("no --path");
    assert!(matches!(
        bare.cmd,
        TopCmd::Verify { cmd: VerifyCmd::ReadmeCoverage { paths } } if paths.is_empty()
    ));
}
