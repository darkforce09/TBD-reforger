//! Where a tracked path sits, for the documentation gates.
//!
//! **Role:** answers the region questions the gates ask of a repository-relative path: is it
//! inside a folder, inside a code tree, inside the README span, below a folder exempt by its name,
//! a Markdown file, or inside an area the size limit leaves alone. The code trees and the README
//! span are derived from where a path sits, never from a list of top-level folder names: every
//! top-level folder the listing holds is judged from the moment it is tracked.
//!
//! **Position:** reads [`repository_layout`]; called by
//! [`crate::readme_coverage`], [`crate::markdown_placement`] and [`crate::gate_scope`].
//!
//! **Signals & state:** none; pure functions over `/`-separated repository-relative paths.
//!
//! **Invariants:** a path is inside a folder only when it equals the folder or continues it after
//! a `/`, so `tools_extra/x` is never inside `tools`; the empty folder is the repository root and
//! holds every path; a top-level folder is outside the README span or the code trees only through
//! a rule written here (the hidden-folder exemption, the documentation root, the retired top-level
//! folders, the pending-merge area), so a new top-level folder fails closed: it is judged until a
//! rule says otherwise.

use repository_layout::{
    ARCHIVE_DIR, PENDING_MERGE_DIR, RETIRED_TOP_LEVEL_FOLDERS, TICKET_DOCUMENTS_DIR,
    is_retired_top_level_folder,
};
use repository_layout::{
    documentation::DOCUMENTATION_ROOT, documentation::GAP_ANALYSIS, documentation::ROADMAP,
};

/// The file every folder in the README span carries, and the only Markdown file name a code tree
/// may hold.
pub(super) const README: &str = "README.md";

/// Folder names that exempt a folder, with everything below it, from the README rules and the
/// code-tree Markdown rule: test sources and generated output. Generated output carries two exact
/// spellings, `generated` in the Rust and web trees and `Generated` in the Enfusion script trees,
/// whose folder names are capitalised; every other casing is an ordinary folder. A folder whose
/// name starts with `.` (hidden tool configuration) is exempt the same way.
const EXEMPT_FOLDER_NAMES: [&str; 3] = ["tests", "generated", "Generated"];

/// Whether the top-level folder `top_level_folder` is not a code tree: the documentation root,
/// whose documents the size limit judges instead, or a retired top-level folder, which must hold
/// nothing and which `markdown-placement` judges on its own. Every other top-level folder is a
/// code tree, so none is left out by a list that was not extended; hidden top-level folders (tool
/// configuration such as `.github`, `.ai`, `.cursor`) are exempt by [`below_exempt_folder`], and
/// build output such as `target/` is gitignored, so no listing ever holds it.
fn is_outside_the_code_trees(top_level_folder: &str) -> bool {
    top_level_folder == DOCUMENTATION_ROOT || is_retired_top_level_folder(top_level_folder)
}

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

/// Whether the file at `path` lies in a code tree: a top-level folder that is neither the
/// documentation root nor retired ([`is_outside_the_code_trees`]) holds it. A file at the
/// repository root lies in no code tree.
pub(super) fn in_code_tree(path: &str) -> bool {
    path.split_once('/')
        .is_some_and(|(top_level_folder, _)| !is_outside_the_code_trees(top_level_folder))
}

/// Whether `path` lies under the documentation root.
pub(super) fn in_documentation_root(path: &str) -> bool {
    is_within(path, DOCUMENTATION_ROOT)
}

/// Whether `folder` belongs to the README span, the one set of folders both README rules judge:
/// every folder below the repository root, the top-level folders included, minus the exempt
/// folders with everything below them — a folder exempt by its name ([`below_exempt_folder`]),
/// the pending-merge area and the retired top-level folders. A README.md inside an exempt folder
/// is neither required nor checked. The repository root is outside the span: its README.md is the
/// project's front page, and the project instructions map the root.
pub(super) fn in_readme_span(folder: &str) -> bool {
    !folder.is_empty()
        && !below_exempt_folder(folder)
        && !is_within(folder, PENDING_MERGE_DIR)
        && !RETIRED_TOP_LEVEL_FOLDERS
            .iter()
            .any(|(retired, _)| is_within(folder, retired))
}

/// Whether any component of `folder` is a test folder, a generated-output folder or a hidden
/// folder, which exempts it with everything below from the README rules and the code-tree
/// Markdown rule.
pub(super) fn below_exempt_folder(folder: &str) -> bool {
    folder
        .split('/')
        .any(|name| EXEMPT_FOLDER_NAMES.contains(&name) || name.starts_with('.'))
}

/// Whether `path` names a Markdown file: its extension is `md` in any letter case.
pub(super) fn is_markdown(path: &str) -> bool {
    file_name(path)
        .rsplit_once('.')
        .is_some_and(|(stem, extension)| !stem.is_empty() && extension.eq_ignore_ascii_case("md"))
}

/// Whether a document under the documentation root is outside the size limit: frozen ticket
/// records and archives, pending merge sources, and the sync-managed documents.
pub(super) fn is_size_exempt(path: &str) -> bool {
    [TICKET_DOCUMENTS_DIR, ARCHIVE_DIR, PENDING_MERGE_DIR]
        .iter()
        .any(|area| is_within(path, area))
        || SYNC_MANAGED_DOCUMENTS.contains(&path)
}

#[cfg(test)]
#[path = "tests/path_regions.rs"]
mod tests;
