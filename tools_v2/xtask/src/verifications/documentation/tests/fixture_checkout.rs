//! A throwaway checkout on disk for the documentation gate tests.
//!
//! Tracked files are written and listed; untracked files are only written; a listed-only path is
//! in the listing but absent from the disk. The gates judge the listing, never the disk walk, so
//! the three kinds cover every way the two can disagree.

use std::path::{Path, PathBuf};

use verification_core::Verdict;

use super::GateRun;
use super::tracked_tree::TrackedTree;

/// A temporary checkout root plus the listing a `git ls-files` there would print.
pub(super) struct FixtureCheckout {
    root: PathBuf,
    listing: Vec<String>,
}

impl FixtureCheckout {
    /// An empty checkout under the system temporary folder, unique per test tag and process.
    pub(super) fn new(tag: &str) -> FixtureCheckout {
        let root =
            std::env::temp_dir().join(format!("documentation-gates-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create the fixture checkout");
        FixtureCheckout {
            root,
            listing: Vec::new(),
        }
    }

    /// Write `path` and list it as tracked.
    pub(super) fn tracked(&mut self, path: &str, text: &str) -> &mut FixtureCheckout {
        self.write(path, text);
        self.listing.push(path.to_string());
        self
    }

    /// Write `path` without listing it.
    pub(super) fn untracked(&mut self, path: &str, text: &str) -> &mut FixtureCheckout {
        self.write(path, text);
        self
    }

    /// List `path` as tracked without writing it.
    pub(super) fn listed_only(&mut self, path: &str) -> &mut FixtureCheckout {
        self.listing.push(path.to_string());
        self
    }

    /// The tree the listing describes.
    pub(super) fn tree(&self) -> TrackedTree {
        TrackedTree::from_listing(&self.listing.join("\0"))
    }

    /// The checkout root.
    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    fn write(&self, path: &str, text: &str) {
        let full = self.root.join(path);
        if let Some(folder) = full.parent() {
            std::fs::create_dir_all(folder).expect("create a fixture folder");
        }
        std::fs::write(&full, text).expect("write a fixture file");
    }
}

impl Drop for FixtureCheckout {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A README whose Contents block is `root` followed by `entries`, between an H1 and a closing
/// section, the shape the README standard asks for.
pub(super) fn readme_with_contents(root: &str, entries: &[&str]) -> String {
    let mut block = vec![root];
    block.extend_from_slice(entries);
    format!(
        "# Fixture\n\nWhat the folder holds.\n\n## Contents\n\n```text\n{}\n```\n\n## Boundaries\n\n\
         - Depends on: nothing.\n",
        block.join("\n")
    )
}

/// The rendered text of every verdict of `run` that did not hold, in order.
pub(super) fn failures(run: &GateRun) -> Vec<String> {
    run.verdicts
        .iter()
        .filter(|verdict| !matches!(verdict, Verdict::Held))
        .map(ToString::to_string)
        .collect()
}

/// How many verdicts of `run` held, broke a rule, and did not run.
pub(super) fn outcome_counts(run: &GateRun) -> (usize, usize, usize) {
    run.verdicts.iter().fold(
        (0, 0, 0),
        |(held, failed, not_run), verdict| match verdict {
            Verdict::Held => (held + 1, failed, not_run),
            Verdict::Failed(_) => (held, failed + 1, not_run),
            Verdict::DidNotRun(..) => (held, failed, not_run + 1),
        },
    )
}
