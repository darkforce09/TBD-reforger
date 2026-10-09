//! The `/// @route METHOD PATH` tags of the API (`src/` and every other API crate's `src/`) and
//! their cross-check against the route table.
//!
//! **Role:** collects every column-0 `@route` tag with the column-0 `pub fn` it documents, and
//! compares the tags with the parsed [`super::route_table::RouteRow`]s in both directions.
//!
//! **Position:** test support; read by the coverage binary's route-table case.
//!
//! **Signals & state:** none; reads source files on each call.
//!
//! **Invariants:** a tag binds to the next column-0 `pub fn` with only comments, attributes,
//! indented lines and blank lines between; any other column-0 line in between orphans it; tag
//! paths written `:param` compare equal to the router's `{param}`; a tag matches a row only on
//! (method, path, handler name) and only when it sits in the handler's module file or
//! directory; a row served by a closure or a file mount needs no tag.

use std::path::{Path, PathBuf};

use super::route_table::{Handler, RouteRow, api_crate_source_roots, crate_source_root};
use super::rust_source_scanning::{module_directory, rust_files_under};

/// One `@route` tag and the handler it documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteTag {
    /// The upper-case method.
    pub method: String,
    /// The path with every `:param` rewritten to `{param}`.
    pub path: String,
    /// The name of the documented `pub fn`.
    pub handler_fn: String,
    /// The file holding the tag: relative to `src/`, or absolute in an API crate.
    pub file: PathBuf,
    /// The tag's 1-based line.
    pub line: usize,
}

impl RouteTag {
    /// `METHOD /path`.
    pub(crate) fn key(&self) -> String {
        format!("{} {}", self.method, self.path)
    }
}

/// Every tag under `src_root`, or the malformed and orphaned tag lines as errors.
pub(crate) fn collect_route_tags(src_root: &Path) -> Result<Vec<RouteTag>, Vec<String>> {
    let mut tags = Vec::new();
    let mut errors = Vec::new();
    scan_tree(
        src_root,
        TagFileSpelling::RelativeToRoot,
        &mut tags,
        &mut errors,
    );
    finish_collection(tags, errors, &src_root.display().to_string())
}

/// Every `@route` tag the API's route table can reach: the tags under this package's `src/`, with
/// files relative to it, and the tags under every other API crate's `src/`, with absolute files —
/// the spelling the route table gives a handler module in each. This package sits under
/// `crates/api/` too, and [`api_crate_source_roots`] leaves it out, so each tag is collected once.
/// The handlers live in the API crates, so `src/` alone may hold no tag; the collection fails only
/// when the whole set is empty.
pub(crate) fn collect_api_route_tags() -> Result<Vec<RouteTag>, Vec<String>> {
    let mut tags = Vec::new();
    let mut errors = Vec::new();
    let package_root = crate_source_root();
    scan_tree(
        &package_root,
        TagFileSpelling::RelativeToRoot,
        &mut tags,
        &mut errors,
    );
    for root in api_crate_source_roots().map_err(|error| vec![error])? {
        scan_tree(&root, TagFileSpelling::Absolute, &mut tags, &mut errors);
    }
    let scope = format!("{} or any API crate's src/", package_root.display());
    finish_collection(tags, errors, &scope)
}

/// How a collected tag spells its file.
#[derive(Debug, Clone, Copy)]
enum TagFileSpelling {
    /// Relative to the scanned root (this package's `src/`, or a synthetic tree).
    RelativeToRoot,
    /// The absolute path (an API crate's `src/`).
    Absolute,
}

/// Scan every Rust file under `root`, appending its tags and its malformed or orphaned tag lines.
fn scan_tree(
    root: &Path,
    spelling: TagFileSpelling,
    tags: &mut Vec<RouteTag>,
    errors: &mut Vec<String>,
) {
    for path in rust_files_under(root) {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                errors.push(format!("read {}: {error}", path.display()));
                continue;
            }
        };
        let file = match spelling {
            TagFileSpelling::RelativeToRoot => {
                path.strip_prefix(root).unwrap_or(&path).to_path_buf()
            }
            TagFileSpelling::Absolute => path.clone(),
        };
        scan_file(&file, &text, tags, errors);
    }
}

/// The collected tags, or the errors; an empty, error-free collection is itself an error, since a
/// walk that found no tag has lost the tree and every cross-check would pass vacuously.
fn finish_collection(
    tags: Vec<RouteTag>,
    mut errors: Vec<String>,
    scope: &str,
) -> Result<Vec<RouteTag>, Vec<String>> {
    if tags.is_empty() && errors.is_empty() {
        errors.push(format!("no @route tag under {scope}"));
    }
    if errors.is_empty() {
        Ok(tags)
    } else {
        Err(errors)
    }
}

fn scan_file(file: &Path, text: &str, tags: &mut Vec<RouteTag>, errors: &mut Vec<String>) {
    let mut pending: Vec<(String, String, usize)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if let Some(rest) = line.strip_prefix("///") {
            if let Some(tag) = rest.trim_start().strip_prefix("@route") {
                let words: Vec<&str> = tag.split_whitespace().collect();
                match words.as_slice() {
                    [method, path] => {
                        pending.push((method.to_uppercase(), normalize_tag_path(path), number))
                    }
                    _ => errors.push(format!("{}:{number}: malformed @route tag", file.display())),
                }
            }
            continue;
        }
        if pending.is_empty() {
            continue;
        }
        if let Some(name) = documented_fn_name(line) {
            for (method, path, line) in pending.drain(..) {
                tags.push(RouteTag {
                    method,
                    path,
                    handler_fn: name.to_string(),
                    file: file.to_path_buf(),
                    line,
                });
            }
        } else if !(line.trim().is_empty()
            || line.starts_with("#[")
            || line.starts_with("//")
            || line.starts_with(' '))
        {
            for (method, path, line) in pending.drain(..) {
                errors.push(format!(
                    "{}:{line}: @route {method} {path} documents no pub fn",
                    file.display()
                ));
            }
        }
    }
    for (method, path, line) in pending {
        errors.push(format!(
            "{}:{line}: @route {method} {path} documents no pub fn",
            file.display()
        ));
    }
}

/// The name of a column-0 `pub fn` / `pub async fn` declared on `line`.
fn documented_fn_name(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("pub ")?.trim_start();
    let rest = rest.strip_prefix("async ").map_or(rest, str::trim_start);
    let rest = rest.strip_prefix("fn ")?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

/// `:param` segments rewritten to the router's `{param}` spelling; the name is kept, so a tag
/// naming another parameter still differs.
pub(crate) fn normalize_tag_path(path: &str) -> String {
    path.split('/')
        .map(|segment| match segment.strip_prefix(':') {
            Some(name) => format!("{{{name}}}"),
            None => segment.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn tag_in_module(tag: &RouteTag, module: &Path) -> bool {
    tag.file == module || module_directory(module).is_some_and(|dir| tag.file.starts_with(dir))
}

/// Every disagreement between rows and tags: a named handler row with no tag, a tag with no
/// row, and a tag whose route exists but whose file is outside the handler's module.
pub(crate) fn cross_check(rows: &[RouteRow], tags: &[RouteTag]) -> Vec<String> {
    let mut problems = Vec::new();
    for row in rows {
        let Handler::Function { name, module } = &row.handler else {
            continue;
        };
        let matching: Vec<&RouteTag> = tags
            .iter()
            .filter(|tag| tag.key() == row.key() && &tag.handler_fn == name)
            .collect();
        if matching.is_empty() {
            problems.push(format!(
                "untagged route: {} served by {name} ({}) carries no @route tag",
                row.key(),
                module.display()
            ));
        } else if !matching.iter().any(|tag| tag_in_module(tag, module)) {
            problems.push(format!(
                "misplaced tag: {} is served by {name} in {}, but its tag sits in {}",
                row.key(),
                module.display(),
                matching[0].file.display()
            ));
        }
    }
    for tag in tags {
        let served = rows
            .iter()
            .any(|row| row.key() == tag.key() && row.handler_fn() == Some(tag.handler_fn.as_str()));
        if !served {
            problems.push(format!(
                "orphan tag: {}:{} claims {} on {}, which no route table registers",
                tag.file.display(),
                tag.line,
                tag.key(),
                tag.handler_fn
            ));
        }
    }
    problems
}
