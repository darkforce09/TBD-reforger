//! Tests for [`super`] — member expansion — and for the workspace keys of
//! [`crate::cargo_manifest`] the member reader and the workspace laws read.

use super::*;
use crate::cargo_manifest::{DependencyKind, LintsSource, parse_manifest};
use crate::temporary_checkout::{TemporaryCheckout, this_repository};

fn checkout(name: &str, members: &str) -> TemporaryCheckout {
    let checkout = TemporaryCheckout::empty(&format!("members-{name}"));
    checkout.write(
        "Cargo.toml",
        &format!("[workspace]\nmembers = [\n{members}\n]\nexclude = [\"crates/api/excluded\"]\n"),
    );
    for (path, name) in [
        ("apps/server", "api"),
        ("crates/foundation/newtype_ids", "newtype_ids"),
        ("crates/api/api_state", "api_state"),
        ("crates/api/excluded", "excluded"),
        ("crates/frontend/pages/account_pages", "account_pages"),
    ] {
        checkout.write(
            &format!("{path}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\n"),
        );
    }
    checkout
}

#[test]
fn workspace_members_expands_explicit_paths_and_globs_minus_excludes() {
    let checkout = checkout(
        "globs",
        "    \"apps/server\", # the server\n    \"crates/*/*\",\n    \"crates/frontend/*/*\",",
    );
    let members = read_workspace_members(checkout.root()).expect("the reader runs");
    let paths: Vec<&str> = members.iter().map(|m| m.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "apps/server",
            "crates/api/api_state",
            "crates/foundation/newtype_ids",
            "crates/frontend/pages/account_pages"
        ]
    );
    let pages = &members[3];
    assert_eq!(
        (
            pages.package_name.as_str(),
            pages.folder_name(),
            pages.parent_folder()
        ),
        ("account_pages", "account_pages", "crates/frontend/pages")
    );
}

#[test]
fn workspace_members_a_missing_member_folder_or_root_manifest_did_not_run() {
    let checkout = checkout("missing", "\"apps/server\", \"apps/gone\"");
    assert!(
        matches!(read_workspace_members(checkout.root()), Err(NotRun::TargetMissing(path)) if path.ends_with("apps/gone"))
    );
    checkout.write("Cargo.toml", "[package]\nname = \"not_a_workspace\"\n");
    assert!(matches!(
        read_workspace_members(checkout.root()),
        Err(NotRun::TargetMissing(_))
    ));
    std::fs::remove_file(checkout.root().join("Cargo.toml")).unwrap();
    assert!(matches!(
        read_workspace_members(checkout.root()),
        Err(NotRun::TargetMissing(_))
    ));
    let checkout = checkout_without_manifest();
    assert!(
        matches!(read_workspace_members(checkout.root()), Err(NotRun::TargetMissing(path)) if path.ends_with("apps/empty/Cargo.toml"))
    );
}

fn checkout_without_manifest() -> TemporaryCheckout {
    let checkout = TemporaryCheckout::empty("members-no-manifest");
    checkout.write("Cargo.toml", "[workspace]\nmembers = [\"apps/empty\"]\n");
    checkout.write("apps/empty/README.md", "# Empty\n");
    checkout
}

#[test]
fn workspace_members_wildcards_match_whole_names() {
    assert!(wildcard_matches("*", "anything"));
    assert!(wildcard_matches("api_*", "api_state"));
    assert!(wildcard_matches("*_pages", "account_pages"));
    assert!(wildcard_matches("a?c", "abc"));
    assert!(!wildcard_matches("api_*", "map_api"));
    assert!(!wildcard_matches("a?c", "abbc"));
}

#[test]
fn workspace_members_reader_reads_layout_lints_inheritance_and_targets() {
    let manifest = parse_manifest(
        r#"
[package]
name = "frontend_ui"
edition.workspace = true
rust-version = { workspace = true }
license = "UNLICENSED"

[package.metadata.layout]
category = "crates/frontend/foundation" # the layer
tier = 2
targets = "any"

[lints]
workspace = true

[lib]
path = "src/lib.rs"

[[bin]]
name = "tool"

[dependencies]
serde = { workspace = true, features = ["derive"] }
regex.workspace = true
regex.features = ["std"]
leptos = "0.8"

[dependencies.newtype_ids]
workspace = true

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { workspace = true }

[build-dependencies]
cc = "1"

[dev-dependencies]
frontend_ui = { path = ".", features = ["test_fixtures"] }

[features]
test_fixtures = []
"#,
    );
    let layout = manifest.layout.as_ref().expect("the layout table is read");
    assert_eq!(
        (
            layout.category.as_deref(),
            layout.tier_number(),
            layout.targets.as_deref()
        ),
        (Some("crates/frontend/foundation"), Some(2), Some("any"))
    );
    assert_eq!(manifest.lints, LintsSource::Workspace);
    assert!(manifest.package_field("edition").unwrap().inherited);
    assert!(manifest.package_field("rust-version").unwrap().inherited);
    assert!(!manifest.package_field("license").unwrap().inherited);
    assert_eq!(
        manifest
            .library
            .as_ref()
            .and_then(|lib| lib.path.as_deref()),
        Some("src/lib.rs")
    );
    assert_eq!(manifest.binaries[0].name.as_deref(), Some("tool"));
    let edges: Vec<(&str, DependencyKind, bool, Option<&str>)> = manifest
        .dependencies
        .iter()
        .map(|e| {
            (
                e.package.as_str(),
                e.kind,
                e.from_workspace,
                e.target_cfg.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        edges,
        [
            ("serde", DependencyKind::Normal, true, None),
            ("regex", DependencyKind::Normal, true, None),
            ("leptos", DependencyKind::Normal, false, None),
            ("newtype_ids", DependencyKind::Normal, true, None),
            (
                "web-sys",
                DependencyKind::Normal,
                true,
                Some("cfg(target_arch = \"wasm32\")")
            ),
            ("cc", DependencyKind::Build, false, None),
            ("frontend_ui", DependencyKind::Development, false, None),
        ]
    );
    assert_eq!(manifest.dependencies[1].features, ["std"]);
}

#[test]
fn workspace_members_reader_keeps_workspace_dependencies_out_of_the_edges() {
    let root = parse_manifest(
        "[workspace]\nmembers = [\"a\"]\n\n[workspace.dependencies]\nserde = \"1\"\n\n[workspace.dependencies.regex]\nversion = \"1\"\n\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n",
    );
    assert!(root.dependencies.is_empty());
    assert_eq!(root.workspace.unwrap().members, ["a"]);
    assert_eq!(root.lints, LintsSource::Absent);
}

#[test]
fn workspace_members_this_checkout_has_every_listed_member() {
    let members = read_workspace_members(&this_repository()).expect("the root manifest reads");
    assert!(members.len() >= 11, "{members:#?}");
    assert!(members.iter().all(|member| !member.package_name.is_empty()));
}
