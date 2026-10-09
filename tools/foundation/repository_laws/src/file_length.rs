//! The size advice: a production file holds at most 500 lines.
//!
//! **Role:** counts the raw lines of every production `.rs` and `.c` file under the law roots and
//! reports each file over [`PRODUCTION_MAX_LINES`], together with the rendered lines
//! `cargo xtask verify file-length` prints as warnings.
//! **Position:** reads [`super::source_roots`]; consumed by the `verify file-length` gate in
//! `tools/checks/repository_checks`.
//! **Signals & state:** none; pure functions over the checkout.
//! **Invariants:** test files ([`super::source_roots::is_test_file`]) are never counted. An
//! unreadable file is [`NotRun::Unreadable`], never a file of zero lines, and the walked file list
//! is returned so a caller can refuse an empty walk.

use std::path::{Path, PathBuf};

use super::source_roots::{is_test_file, repository_relative, walk_length_gated_sources};
use verification_core::verdict::NotRun;

/// Most lines a production file should hold.
pub const PRODUCTION_MAX_LINES: usize = 500;

/// One production file over [`PRODUCTION_MAX_LINES`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLengthViolation {
    /// Repository-relative path with `/` separators.
    pub path: String,
    /// Raw line count, as [`str::lines`] counts them.
    pub lines: usize,
}

impl FileLengthViolation {
    /// The warning line the file-length gate prints for this file.
    pub fn rendered(&self) -> String {
        format!(
            "warning: {} is {} lines (>{PRODUCTION_MAX_LINES}); consider splitting it by \
             responsibility.",
            self.path, self.lines
        )
    }
}

/// Everything one walk of the size advice found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLengthScan {
    /// Every production file the walk counted, sorted.
    pub files: Vec<PathBuf>,
    /// Every file over [`PRODUCTION_MAX_LINES`], in walk order.
    pub violations: Vec<FileLengthViolation>,
}

impl FileLengthScan {
    /// The gate's summary line: total files, the per-extension split and the over-long count.
    pub fn summary(&self) -> String {
        let with_extension = |wanted: &str| {
            self.files
                .iter()
                .filter(|path| path.extension().is_some_and(|ext| ext == wanted))
                .count()
        };
        format!(
            "file-length: scanned {} production source file(s) ({} .rs, {} .c), {} over \
             {PRODUCTION_MAX_LINES} lines.",
            self.files.len(),
            with_extension("rs"),
            with_extension("c"),
            self.violations.len()
        )
    }
}

/// Count every production file under the law roots of `repo_root` and report each one over
/// [`PRODUCTION_MAX_LINES`].
pub fn scan_file_lengths(repo_root: &Path) -> Result<FileLengthScan, NotRun> {
    let mut files = Vec::new();
    let mut violations = Vec::new();
    for file in walk_length_gated_sources(repo_root)? {
        let path = repository_relative(repo_root, &file);
        if is_test_file(&path) {
            continue;
        }
        let lines = match std::fs::read_to_string(&file) {
            Ok(text) => text.lines().count(),
            Err(source) => {
                return Err(NotRun::Unreadable {
                    path: file.clone(),
                    source,
                });
            }
        };
        if lines > PRODUCTION_MAX_LINES {
            violations.push(FileLengthViolation { path, lines });
        }
        files.push(file);
    }
    Ok(FileLengthScan { files, violations })
}
