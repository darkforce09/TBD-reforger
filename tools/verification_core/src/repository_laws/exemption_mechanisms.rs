//! The no-exemption law: the size and test-placement rules have no way around them.
//!
//! **Role:** finds the three shapes an exemption from the structural laws takes — a file that
//! lists exempt paths, a comment directive that switches a rule off for a file, and a declared
//! table type or constant of exempted paths — anywhere under the law roots, plus exemption files at
//! the repository root.
//! **Position:** reads [`super::source_roots`]; consumed by the `engineering_laws` test binary of
//! `website-api`.
//! **Signals & state:** none; pure functions over the checkout.
//! **Invariants:** the patterns are anchored on the law-7 subjects (file length, size, line
//! limit, inline or sibling tests) or on words that only ever name an exemption table, so an
//! allow-list of hosts or a rate-limit exemption is not a finding. Directives are read only on
//! comment lines, so the gate messages that say "no exemptions" are not findings either.

use std::path::Path;

use super::source_roots::{repository_relative, walk_law_sources};
use crate::pattern::Pattern;
use crate::verdict::NotRun;

/// A file name that is an exemption list: `allowlist`, `allow_list`, `allow-list`, `exemption`,
/// `exemptions` or a `grandfather…` stem, optionally prefixed by the rule it would exempt, with
/// any extension.
pub const EXEMPTION_FILE_NAME_RE: &str = r"(?i)^\.?((coding|file|size|length|line|test|doc)[-_]?\w*[-_])?(allow[-_]?list|grandfather\w*|exemptions?)\.[a-z0-9]+$";

/// A comment directive that would switch a structural rule off: the rule's name followed by
/// `:` or `=` and an off-switch word, or an off-switch word hyphenated onto the rule's name.
pub const EXEMPTION_DIRECTIVE_RE: &str = r"(?i)\b(file[-_ ]?length|size[-_ ]?3|line[-_ ]limit|coding[-_ ]standards|inline[-_ ]tests?|sibling[-_ ]tests?|test[-_ ]placement)\s*[:=]\s*(allow|allowed|exempt|exempted|ignore|skip|off|disable|disabled)\b|\b(allow|exempt|ignore|skip|disable)[-_](file[-_]length|size[-_]?3|long[-_]file|inline[-_]tests?|line[-_]limit)\b";

/// A declared table of exempted paths: a `const`, `static`, `struct`, `enum` or `type` whose name
/// carries `grandfather` or `allowlist`.
pub const EXEMPTION_TABLE_RE: &str =
    r"(?i)\b(const|static|struct|enum|type)\s+\w*(grandfather|allow_?list)\w*";

/// Which exemption shape a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExemptionKind {
    /// A file whose name makes it an exemption list ([`EXEMPTION_FILE_NAME_RE`]).
    ExemptionFile,
    /// A comment line that switches a rule off ([`EXEMPTION_DIRECTIVE_RE`]).
    CommentDirective,
    /// A declared table of exempted paths ([`EXEMPTION_TABLE_RE`]).
    ExemptionTable,
}

/// One exemption mechanism found in the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExemptionFinding {
    /// Which shape it is.
    pub kind: ExemptionKind,
    /// Repository-relative path with `/` separators.
    pub path: String,
    /// 1-based line of a directive or table; `None` for a whole file.
    pub line_no: Option<usize>,
    /// The offending line, or the file name.
    pub text: String,
}

impl ExemptionFinding {
    /// `path[:line]: <kind> — text`.
    pub fn rendered(&self) -> String {
        let place = match self.line_no {
            Some(line_no) => format!("{}:{line_no}", self.path),
            None => self.path.clone(),
        };
        format!("{place}: {:?} — {}", self.kind, self.text.trim())
    }
}

/// Everything one walk of the no-exemption law found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExemptionScan {
    /// How many files the walk examined, of any extension.
    pub files_examined: usize,
    /// Every exemption mechanism, files first, then lines in walk order.
    pub findings: Vec<ExemptionFinding>,
}

/// Scan the law roots of `repo_root`, and the files directly in `repo_root`, for exemption
/// mechanisms.
pub fn scan_exemption_mechanisms(repo_root: &Path) -> Result<ExemptionScan, NotRun> {
    let file_name = compiled(EXEMPTION_FILE_NAME_RE)?;
    let directive = compiled(EXEMPTION_DIRECTIVE_RE)?;
    let table = compiled(EXEMPTION_TABLE_RE)?;

    let mut files = walk_law_sources(repo_root, |_| true)?;
    files.extend(top_level_files(repo_root)?);
    let mut findings = Vec::new();
    for file in &files {
        let name = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if file_name.is_match(&name) {
            findings.push(ExemptionFinding {
                kind: ExemptionKind::ExemptionFile,
                path: repository_relative(repo_root, file),
                line_no: None,
                text: name,
            });
        }
    }
    for file in &files {
        let is_source = file
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "c");
        if !is_source {
            continue;
        }
        let text = std::fs::read_to_string(file).map_err(|source| NotRun::Unreadable {
            path: file.clone(),
            source,
        })?;
        for (index, line) in text.lines().enumerate() {
            let kind = if is_comment_line(line) && directive.is_match(line) {
                Some(ExemptionKind::CommentDirective)
            } else if table.is_match(line) {
                Some(ExemptionKind::ExemptionTable)
            } else {
                None
            };
            if let Some(kind) = kind {
                findings.push(ExemptionFinding {
                    kind,
                    path: repository_relative(repo_root, file),
                    line_no: Some(index + 1),
                    text: line.to_string(),
                });
            }
        }
    }
    Ok(ExemptionScan {
        files_examined: files.len(),
        findings,
    })
}

/// True when `line` is a comment line: `//`, `/*` or a `*` continuation, after indentation.
pub fn is_comment_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

/// The regular files directly inside `repo_root`, where an exemption file for the whole tree
/// would sit.
fn top_level_files(repo_root: &Path) -> Result<Vec<std::path::PathBuf>, NotRun> {
    let entries = std::fs::read_dir(repo_root).map_err(|source| NotRun::Unreadable {
        path: repo_root.to_path_buf(),
        source,
    })?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| NotRun::Unreadable {
            path: repo_root.to_path_buf(),
            source,
        })?;
        let is_file = entry
            .file_type()
            .map_err(|source| NotRun::Unreadable {
                path: entry.path(),
                source,
            })?
            .is_file();
        if is_file {
            files.push(entry.path());
        }
    }
    files.sort();
    Ok(files)
}

/// Compile one of this module's patterns; a pattern that does not compile is a check that
/// cannot run.
fn compiled(pattern: &str) -> Result<Pattern, NotRun> {
    Pattern::regex(pattern).map_err(|error| NotRun::ToolError {
        tool: "regex".into(),
        status: 2,
        stderr: error.to_string(),
    })
}

#[cfg(test)]
#[path = "tests/exemption_mechanisms.rs"]
mod tests;
