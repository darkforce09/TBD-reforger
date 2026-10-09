//! A throwaway Cargo workspace for the workspace-law tests.
//!
//! **Role:** plants a root manifest and member crates in a [`TemporaryCheckout`], each member
//! either written verbatim or built as a library crate that keeps every layout and anatomy rule,
//! so a test changes exactly the one thing it is about.
//! **Position:** test support for the sibling test modules of [`super`].
//! **Signals & state:** each [`FixtureWorkspace`] owns one checkout and its member list; the root
//! manifest is rewritten on every member added.
//! **Invariants:** a layout crate's package name is its folder name and its category is its
//! parent folder, so an unchanged layout crate is green under every law.

use std::path::Path;

use crate::temporary_checkout::TemporaryCheckout;

/// A temporary workspace.
pub(super) struct FixtureWorkspace {
    checkout: TemporaryCheckout,
    members: Vec<String>,
}

/// One dependency of a layout crate: the package name and the table it sits in.
pub(super) struct Dependency<'a> {
    pub(super) package: &'a str,
    pub(super) table: &'a str,
}

/// A `[dependencies]` edge on `package`.
pub(super) fn normal(package: &str) -> Dependency<'_> {
    Dependency {
        package,
        table: "dependencies",
    }
}

impl FixtureWorkspace {
    /// An empty workspace whose checkout is named after `name`.
    pub(super) fn new(name: &str) -> Self {
        let workspace = Self {
            checkout: TemporaryCheckout::empty(&format!("workspace-{name}")),
            members: Vec::new(),
        };
        workspace.write_root_manifest();
        workspace
    }

    /// The repository root.
    pub(super) fn root(&self) -> &Path {
        self.checkout.root()
    }

    /// Write `body` to the repository-relative `rel`.
    pub(super) fn write(&self, rel: &str, body: &str) {
        self.checkout.write(rel, body);
    }

    /// Add the member at `path` with the manifest `manifest`, written verbatim.
    pub(super) fn member(&mut self, path: &str, manifest: &str) {
        self.write(&format!("{path}/Cargo.toml"), manifest);
        self.members.push(path.to_string());
        self.write_root_manifest();
    }

    /// Add a library crate at `path` that keeps every layout and anatomy rule, at `tier` with
    /// `targets`, depending on `dependencies` (each `{ workspace = true }`).
    pub(super) fn layout_crate(
        &mut self,
        path: &str,
        tier: u32,
        targets: &str,
        dependencies: &[Dependency<'_>],
    ) {
        let (category, name) = path
            .rsplit_once('/')
            .expect("a layout crate has a category");
        let mut manifest = format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition.workspace = true\n\
             rust-version.workspace = true\n\n[package.metadata.layout]\ncategory = \"{category}\"\n\
             tier = {tier}\ntargets = \"{targets}\"\n\n[lints]\nworkspace = true\n"
        );
        let mut tables: Vec<&str> = dependencies.iter().map(|d| d.table).collect();
        tables.dedup();
        for table in tables {
            manifest.push_str(&format!("\n[{table}]\n"));
            for dependency in dependencies.iter().filter(|d| d.table == table) {
                manifest.push_str(&format!(
                    "{} = {{ workspace = true }}\n",
                    dependency.package
                ));
            }
        }
        self.member(path, &manifest);
        self.write(
            &format!("{path}/README.md"),
            &format!("# {name}\n\n## Contents\n\n```text\n{path}/\n```\n"),
        );
        self.write(
            &format!("{path}/src/lib.rs"),
            "//! A fixture crate.\n\npub mod prelude;\n",
        );
        self.write(&format!("{path}/src/prelude.rs"), "//! Nothing yet.\n");
    }

    /// Rewrite the root manifest with the current member list.
    fn write_root_manifest(&self) {
        let members: Vec<String> = self
            .members
            .iter()
            .map(|m| format!("    \"{m}\",\n"))
            .collect();
        self.write(
            "Cargo.toml",
            &format!(
                "[workspace]\nresolver = \"3\"\nmembers = [\n{}]\n\n[workspace.package]\n\
                 edition = \"2024\"\n",
                members.concat()
            ),
        );
    }
}

/// A green workspace for the crate-tier tests: two foundation crates, a mission crate, a tool
/// foundation crate and the application `crates/api/api_server` over the mission crate.
pub(super) fn green_workspace(name: &str) -> FixtureWorkspace {
    let mut workspace = FixtureWorkspace::new(name);
    workspace.layout_crate("crates/foundation/newtype_ids", 0, "any", &[]);
    workspace.layout_crate(
        "crates/foundation/time_source",
        1,
        "any",
        &[normal("newtype_ids")],
    );
    workspace.layout_crate(
        "crates/mission/mission_model",
        2,
        "any",
        &[normal("time_source")],
    );
    workspace.layout_crate("tools/foundation/repository_layout", 0, "any", &[]);
    workspace.layout_crate(
        "crates/api/api_server",
        3,
        "any",
        &[normal("mission_model")],
    );
    workspace
}

/// A plain application manifest named `name` with the `[dependencies]` lines `dependencies`.
pub(super) fn application_manifest(name: &str, dependencies: &str) -> String {
    format!("[package]\nname = \"{name}\"\nedition = \"2024\"\n\n[dependencies]\n{dependencies}")
}
