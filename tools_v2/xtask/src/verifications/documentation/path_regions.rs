//! Where a tracked path sits, for the documentation gates.
//!
//! **Role:** answers the region questions the gates ask of a repository-relative path: is it
//! inside a folder, inside a code tree, inside the README span, below a folder the README rule
//! skips, a Markdown file, or inside an area the size limit leaves alone.
//!
//! **Position:** reads [`crate::core::repository_layout::documentation`]; called by
//! [`super::readme_coverage`], [`super::markdown_placement`] and [`super::gate_scope`].
//!
//! **Signals & state:** none; pure functions over `/`-separated repository-relative paths.
//!
//! **Invariants:** a path is inside a folder only when it equals the folder or continues it after
//! a `/`, so `apps_extra/x` is never inside `apps`; the empty folder is the repository root and
//! holds every path.

use crate::core::repository_layout::documentation::{
    ARCHIVE_DIR, CODE_TREES, DOCUMENTATION_ROOT, GAP_ANALYSIS, PENDING_MERGE_DIR,
    PROGRAM_RECORDS_PREFIX, ROADMAP, TICKET_DOCUMENTS_DIR,
};

/// The file every folder in the README span carries, and the only Markdown file name a code tree
/// may hold.
pub(super) const README: &str = "README.md";

/// Folder names whose subtrees the README rule and the code-tree Markdown rule skip: test sources
/// and generated output. A folder whose name starts with `.` (hidden tool configuration) is
/// skipped the same way.
const SKIPPED_FOLDER_NAMES: [&str; 2] = ["tests", "generated"];

/// Documents `cargo xtask ticket sync` rewrites between markers. Their sync-managed tables stay
/// in one file whatever their length, so the size limit skips them.
const SYNC_MANAGED_DOCUMENTS: [&str; 2] = [ROADMAP, GAP_ANALYSIS];

/// Whether `path` is `folder` itself or lies below it. The empty folder is the repository root.
pub(super) fn is_within(path: &str, folder: &str) -> bool {
    folder.is_empty()
        || path
            .strip_prefix(folder)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The folder holding `path`; a top-level path sits in the repository root, `""`.
pub(super) fn parent_folder(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// The last component of `path`.
pub(super) fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

/// `folder/name`, or `name` alone in the repository root.
pub(super) fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

/// Whether `path` lies in one of the code trees.
pub(super) fn in_code_tree(path: &str) -> bool {
    CODE_TREES.iter().any(|tree| is_within(path, tree))
}

/// Whether `path` lies under the documentation root.
pub(super) fn in_documentation_root(path: &str) -> bool {
    is_within(path, DOCUMENTATION_ROOT)
}

/// Whether `folder` belongs to the README span: a code tree or the documentation root, the roots
/// themselves included, outside the pending-merge area.
pub(super) fn in_readme_span(folder: &str) -> bool {
    (in_code_tree(folder) || in_documentation_root(folder)) && !is_within(folder, PENDING_MERGE_DIR)
}

/// Whether any component of `folder` is a test folder, a generated-output folder or a hidden
/// folder, which the README rule and the code-tree Markdown rule skip with everything below.
pub(super) fn below_skipped_folder(folder: &str) -> bool {
    folder
        .split('/')
        .any(|name| SKIPPED_FOLDER_NAMES.contains(&name) || name.starts_with('.'))
}

/// Whether `path` names a Markdown file: its extension is `md` in any letter case.
pub(super) fn is_markdown(path: &str) -> bool {
    file_name(path)
        .rsplit_once('.')
        .is_some_and(|(stem, extension)| !stem.is_empty() && extension.eq_ignore_ascii_case("md"))
}

/// Whether a document under the documentation root is outside the size limit: frozen ticket
/// records and archives, pending merge sources, the program records at the root, and the
/// sync-managed documents.
pub(super) fn is_size_exempt(path: &str) -> bool {
    [TICKET_DOCUMENTS_DIR, ARCHIVE_DIR, PENDING_MERGE_DIR]
        .iter()
        .any(|area| is_within(path, area))
        || path.starts_with(PROGRAM_RECORDS_PREFIX)
        || SYNC_MANAGED_DOCUMENTS.contains(&path)
}

#[cfg(test)]
#[path = "tests/path_regions.rs"]
mod tests;
