//! The write guard for every `enf` command whose output is a reference lane.
//!
//! **Role:** Resolves an output folder and accepts it only inside the references folder
//! ([`::repository_layout::REFERENCES_DIR`]), and removes a previous output only when the
//! operator asked for it with `--replace`.
//!
//! **Position:** Called by [`crate::enfusion_tooling::cli`] before `enf carve`, `enf extract` and
//! `enf source` write; the writers themselves receive the checked folder.
//!
//! **Signals & state:** none; reads the filesystem, and removes a folder only in
//! [`clear_previous_output`] with `replace` set.
//!
//! **Invariants:** an accepted folder lies strictly below an existing references folder and its
//! path holds no `..`, so `create_dir_all` on it never creates the references folder or a lane
//! anywhere else; a folder holding files is never removed without `replace`, and a removal that
//! fails is an error, never ignored.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Accept `out` as a lane output folder of the checkout found from the working directory.
///
/// A relative `out` resolves against the working directory, as the clap defaults do.
pub fn checked_reference_output(out: &Path) -> Result<PathBuf> {
    let root = ::repository_layout::find_repository_root()?;
    let cwd = std::env::current_dir().context("reading the working directory")?;
    reference_output_within(&root.join(::repository_layout::REFERENCES_DIR), &cwd, out)
}

/// Accept `out`, resolved against `cwd`, only strictly inside the existing folder `references`.
pub fn reference_output_within(references: &Path, cwd: &Path, out: &Path) -> Result<PathBuf> {
    if !references.is_dir() {
        bail!(
            "{} is missing; a reference lane is written only inside it (its README.md is tracked, \
             so run from a checkout of this repository)",
            references.display()
        );
    }
    let resolved = cwd.join(out);
    if resolved.components().any(|c| c == Component::ParentDir) {
        bail!(
            "refusing output folder {}: `..` is not accepted",
            resolved.display()
        );
    }
    if resolved == references || !resolved.starts_with(references) {
        bail!(
            "refusing output folder {}: reference lanes are written only inside {}",
            resolved.display(),
            references.display()
        );
    }
    Ok(resolved)
}

/// Make room for a fresh output in `dir`.
///
/// An absent or empty folder needs nothing. A folder holding files is removed only when
/// `replace` is set, and the removal is announced; otherwise the command refuses.
pub fn clear_previous_output(dir: &Path, replace: bool) -> Result<()> {
    let holds_files = match std::fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_some(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e).with_context(|| format!("reading {}", dir.display())),
    };
    if !holds_files {
        return Ok(());
    }
    if !replace {
        bail!(
            "{} already holds a previous output; pass --replace to remove it and write a fresh one",
            dir.display()
        );
    }
    println!("removing the previous output in {}", dir.display());
    std::fs::remove_dir_all(dir).with_context(|| format!("removing {}", dir.display()))
}

#[cfg(test)]
#[path = "tests/reference_output/tests.rs"]
mod tests;
