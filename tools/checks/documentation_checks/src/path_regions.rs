//! Where a tracked path sits, for the link check.
//!
//! **Role:** answers the path questions the link check asks of a repository-relative path: is it
//! inside a folder, which folder holds it, what its file name is, and whether it is a Markdown
//! file.
//!
//! **Position:** called by [`crate::gate_scope`] and the link check's document and target
//! modules.
//!
//! **Signals & state:** none; pure functions over `/`-separated repository-relative paths.
//!
//! **Invariants:** a path is inside a folder only when it equals the folder or continues it after
//! a `/`, so `tools_extra/x` is never inside `tools`; the empty folder is the repository root and
//! holds every path.

/// The file name of a folder's README, which the link check judges wherever it sits.
pub(super) const README: &str = "README.md";

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

/// Whether `path` names a Markdown file: its extension is `md` in any letter case.
pub(super) fn is_markdown(path: &str) -> bool {
    file_name(path)
        .rsplit_once('.')
        .is_some_and(|(stem, extension)| !stem.is_empty() && extension.eq_ignore_ascii_case("md"))
}
