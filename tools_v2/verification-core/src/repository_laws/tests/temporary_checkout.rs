//! A throwaway checkout for the repository-law tests.
//!
//! **Role:** builds a repository skeleton in the system temporary directory with every law root
//! present — a root manifest naming [`FIXTURE_WORKSPACE_MEMBERS`], each member with a manifest and
//! one `src/lib.rs`, and every pinned script root — so a test plants exactly the files it is about
//! and nothing refuses for a missing root.
//! **Position:** test support for the sibling test modules of [`super`].
//! **Signals & state:** each [`TemporaryCheckout`] owns one directory and removes it on drop.
//! **Invariants:** the directory name carries the process id and the test's own name, so tests
//! running in parallel never share a checkout.

use std::path::{Path, PathBuf};

use super::source_roots::PINNED_SCRIPT_ROOTS;

/// The member folders of the workspace a [`TemporaryCheckout::with_law_roots`] checkout declares:
/// the folders the repository's own members sit in.
pub(super) const FIXTURE_WORKSPACE_MEMBERS: &[&str] = &[
    "apps/fleet_host_agent",
    "apps/ticketboard",
    "apps/website/api_v2",
    "apps/website/frontend",
    "apps/website/map-engine",
    "apps/website/graphics-engine",
    "apps/website/offline-service-worker",
    "tools_v2/verification-core",
    "tools_v2/ticket-engine",
    "tools_v2/xtask",
    "tools_v2/developer-tools",
];

/// A temporary repository root.
pub(super) struct TemporaryCheckout(PathBuf);

impl TemporaryCheckout {
    /// A checkout holding every law root: a workspace of [`FIXTURE_WORKSPACE_MEMBERS`], each
    /// with a `Cargo.toml` and a one-line `src/lib.rs`, and every pinned script root, empty.
    pub(super) fn with_law_roots(name: &str) -> Self {
        let checkout = Self::empty(name);
        let listed: Vec<String> = FIXTURE_WORKSPACE_MEMBERS
            .iter()
            .map(|member| format!("    \"{member}\",\n"))
            .collect();
        checkout.write(
            "Cargo.toml",
            &format!("[workspace]\nmembers = [\n{}]\n", listed.concat()),
        );
        for member in FIXTURE_WORKSPACE_MEMBERS {
            let package = member.rsplit('/').next().unwrap_or(member);
            checkout.write(
                &format!("{member}/Cargo.toml"),
                &format!("[package]\nname = \"{package}\"\n"),
            );
            checkout.write(&format!("{member}/src/lib.rs"), "fn placeholder() {}\n");
        }
        for root in PINNED_SCRIPT_ROOTS {
            std::fs::create_dir_all(checkout.root().join(root)).unwrap();
        }
        checkout
    }

    /// A checkout holding nothing at all.
    pub(super) fn empty(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "verification-core-laws-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    /// The repository root.
    pub(super) fn root(&self) -> &Path {
        &self.0
    }

    /// Write `body` to the repository-relative `rel`, creating its folders.
    pub(super) fn write(&self, rel: &str, body: &str) {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    /// Write a file of exactly `count` comment lines to `rel`.
    pub(super) fn write_lines(&self, rel: &str, count: usize) {
        self.write(rel, &"// line\n".repeat(count));
    }
}

impl Drop for TemporaryCheckout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The repository this crate is built from: two levels above `tools_v2/verification-core`.
pub(super) fn this_repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("tools_v2/verification-core sits two levels below the repository root")
        .to_path_buf()
}
