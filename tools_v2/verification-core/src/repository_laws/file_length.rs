//! The size law: a production file holds at most 500 lines, a test file at most 1000.
//!
//! **Role:** counts the raw lines of every `.rs` and `.c` file under the law roots and reports
//! each file over its ceiling, together with the rendered lines `cargo xtask verify file-length`
//! prints.
//! **Position:** reads [`super::source_roots`]; consumed by the `verify file-length` gate in
//! `tools_v2/xtask` and by the `engineering_laws` test binary of `website-api`.
//! **Signals & state:** none; pure functions over the checkout.
//! **Invariants:** the ceilings are exactly [`PRODUCTION_MAX_LINES`] and [`TEST_MAX_LINES`] with
//! no exemption of any kind. An unreadable file is [`NotRun::Unreadable`], never a file of zero
//! lines, and the walked file list is returned so a caller can refuse an empty walk.

use std::path::{Path, PathBuf};

use super::source_roots::{is_test_file, repository_relative, walk_length_gated_sources};
use crate::verdict::NotRun;

/// Most lines a production file may hold.
pub const PRODUCTION_MAX_LINES: usize = 500;

/// Most lines a test file ([`super::source_roots::is_test_file`]) may hold.
pub const TEST_MAX_LINES: usize = 1000;

/// One file over its ceiling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLengthViolation {
    /// Repository-relative path with `/` separators.
    pub path: String,
    /// Raw line count, as [`str::lines`] counts them.
    pub lines: usize,
    /// The ceiling that applies to the file: [`PRODUCTION_MAX_LINES`] or [`TEST_MAX_LINES`].
    pub max_lines: usize,
}

impl FileLengthViolation {
    /// The `SIZE-3:` report line the file-length gate prints for this file.
    pub fn rendered(&self) -> String {
        format!(
            "SIZE-3: {} is {} lines (>{}). Hard limit exceeded; decompose by responsibility. No exemptions permitted.",
            self.path, self.lines, self.max_lines
        )
    }

    /// True when the file is a test file, so the ceiling is [`TEST_MAX_LINES`].
    pub fn is_test_file(&self) -> bool {
        self.max_lines == TEST_MAX_LINES
    }
}

/// Everything one walk of the size law found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLengthScan {
    /// Every file the walk counted, sorted.
    pub files: Vec<PathBuf>,
    /// Every file over its ceiling, in walk order.
    pub violations: Vec<FileLengthViolation>,
}

impl FileLengthScan {
    /// The gate's summary line: total files, the per-extension split and the violation count.
    pub fn summary(&self) -> String {
        length_scan_summary(&self.files, self.violations.len())
    }
}

/// The ceiling for the repository-relative path `rel`.
pub fn line_limit(rel: &str) -> usize {
    if is_test_file(rel) {
        TEST_MAX_LINES
    } else {
        PRODUCTION_MAX_LINES
    }
}

/// Count every length-gated file under the law roots of `repo_root` and report each one over its
/// ceiling.
pub fn scan_file_lengths(repo_root: &Path) -> Result<FileLengthScan, NotRun> {
    let files = walk_length_gated_sources(repo_root)?;
    let mut violations = Vec::new();
    for file in &files {
        let path = repository_relative(repo_root, file);
        let lines = match std::fs::read_to_string(file) {
            Ok(text) => text.lines().count(),
            Err(source) => {
                return Err(NotRun::Unreadable {
                    path: file.clone(),
                    source,
                });
            }
        };
        let max_lines = line_limit(&path);
        if lines > max_lines {
            violations.push(FileLengthViolation {
                path,
                lines,
                max_lines,
            });
        }
    }
    Ok(FileLengthScan { files, violations })
}

/// The summary line for `files` with `violations` files over their ceilings.
pub fn length_scan_summary(files: &[PathBuf], violations: usize) -> String {
    let with_extension = |wanted: &str| {
        files
            .iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == wanted))
            .count()
    };
    format!(
        "file-length: scanned {} source file(s) ({} .rs, {} .c), {violations} violation(s).",
        files.len(),
        with_extension("rs"),
        with_extension("c")
    )
}

#[cfg(test)]
#[path = "tests/file_length.rs"]
mod tests;
