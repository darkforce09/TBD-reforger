//! The test-placement law: unit tests live in sibling files, never in an inline test-module body.
//!
//! **Role:** finds every `mod <name> {` body in a production Rust file that is a test module —
//! one named `tests` or `test`, or one whose attributes enable it under `cfg(test)` — so the only
//! test module a production file declares is `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
//! **Position:** reads the Rust sources of [`super::source_roots`]; consumed by the
//! `engineering_laws` test binary of `api`.
//! **Signals & state:** none; pure functions over source text.
//! **Invariants:** the check is line-level and `cfg`-aware: the attributes that apply to a module
//! are the `#[...]` attributes on its own line and on the attribute, doc-comment, comment and blank
//! lines directly above it, and a predicate enables tests when `test` appears in it outside any
//! `not(...)` — `cfg(not(test))` never counts. A module declaration ending in `;` is a sibling
//! file and never counts. Test files ([`super::source_roots::is_test_file`]) are the sibling
//! files themselves and are not scanned. The scan reads raw lines, so Rust source quoted inside a
//! literal counts only when a line of the literal begins with the module keyword.

use std::path::Path;

use super::source_roots::{is_test_file, repository_relative, walk_rust_sources};
use crate::verdict::NotRun;

/// One inline test-module body in a production file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineTestModule {
    /// Repository-relative path with `/` separators.
    pub path: String,
    /// 1-based line of the `mod` keyword.
    pub line_no: usize,
    /// The module's name.
    pub module: String,
}

impl InlineTestModule {
    /// `path:line: inline test module <name>`.
    pub fn rendered(&self) -> String {
        format!(
            "{}:{}: inline test module `{}` — move its body to a sibling tests/ file",
            self.path, self.line_no, self.module
        )
    }
}

/// Everything one walk of the test-placement law found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineTestModuleScan {
    /// How many production Rust files the walk read.
    pub production_files: usize,
    /// Every inline test-module body, in walk order.
    pub findings: Vec<InlineTestModule>,
}

/// Scan every production Rust file under the law roots of `repo_root`.
pub fn scan_inline_test_modules(repo_root: &Path) -> Result<InlineTestModuleScan, NotRun> {
    let files = walk_rust_sources(repo_root)?;
    let mut production_files = 0;
    let mut findings = Vec::new();
    for file in &files {
        let path = repository_relative(repo_root, file);
        if is_test_file(&path) {
            continue;
        }
        let text = std::fs::read_to_string(file).map_err(|source| NotRun::Unreadable {
            path: file.clone(),
            source,
        })?;
        production_files += 1;
        for (line_no, module) in inline_test_modules_in(&text) {
            findings.push(InlineTestModule {
                path: path.clone(),
                line_no,
                module,
            });
        }
    }
    Ok(InlineTestModuleScan {
        production_files,
        findings,
    })
}

/// `(line number, module name)` of every inline test-module body in `text`.
pub fn inline_test_modules_in(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            index += 1;
            continue;
        }
        let (attributes, rest, consumed) = leading_attributes(&lines, index);
        pending.extend(attributes);
        if rest.trim().is_empty() {
            index += consumed;
            continue;
        }
        if let Some(module) = inline_module_name(rest) {
            let by_name = module == "tests" || module == "test";
            if by_name
                || pending
                    .iter()
                    .any(|attribute| attribute_enables_tests(attribute))
            {
                found.push((index + consumed, module));
            }
        }
        pending.clear();
        index += consumed;
    }
    found
}

/// The complete `#[...]` attributes that open the line at `start` (an attribute may continue over
/// following lines until its brackets balance), the text after them on the line where the last
/// one closes, and how many lines they span.
fn leading_attributes<'a>(lines: &[&'a str], start: usize) -> (Vec<String>, &'a str, usize) {
    let mut attributes = Vec::new();
    let mut line_index = start;
    let mut rest = lines[start].trim_start();
    while rest.starts_with("#[") {
        let mut text = String::new();
        let mut depth = 0usize;
        let mut closed_at = None;
        loop {
            for (offset, character) in rest.char_indices() {
                match character {
                    '[' => depth += 1,
                    ']' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            closed_at = Some(offset);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            match closed_at {
                Some(offset) => {
                    text.push_str(&rest[..=offset]);
                    rest = rest[offset + 1..].trim_start();
                    break;
                }
                None if line_index + 1 < lines.len() => {
                    text.push_str(rest);
                    text.push(' ');
                    line_index += 1;
                    rest = lines[line_index].trim_start();
                }
                None => {
                    text.push_str(rest);
                    rest = "";
                    break;
                }
            }
        }
        attributes.push(text);
    }
    (attributes, rest, line_index - start + 1)
}

/// The module name when `code` opens an inline module body (`mod name {`), with any visibility.
fn inline_module_name(code: &str) -> Option<String> {
    let mut rest = code.trim_start();
    if let Some(after) = rest.strip_prefix("pub") {
        rest = match after.trim_start().strip_prefix('(') {
            Some(inner) => inner.split_once(')')?.1,
            None => after,
        };
        if !rest.starts_with(char::is_whitespace) {
            return None;
        }
        rest = rest.trim_start();
    }
    let rest = rest.strip_prefix("mod")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    let (name, tail) = rest.split_at(end);
    (!name.is_empty() && tail.trim_start().starts_with('{')).then(|| name.to_string())
}

/// True when the attribute text `attribute` (`#[...]`) turns its item on under `cfg(test)`:
/// a `cfg` predicate naming `test` outside `not(...)`, or a `cfg_attr` whose conditional
/// attributes include such a `cfg`.
pub fn attribute_enables_tests(attribute: &str) -> bool {
    let inner = attribute
        .trim()
        .trim_start_matches("#[")
        .trim_end_matches(']');
    let tokens = tokenize(inner);
    let mut cursor = 0;
    match parse_meta(&tokens, &mut cursor) {
        Some(meta) => meta_attribute_enables_tests(&meta),
        None => false,
    }
}

/// One parsed attribute meta item: a path, optionally followed by a parenthesised list.
#[derive(Debug)]
struct Meta {
    path: String,
    list: Option<Vec<Meta>>,
}

fn meta_attribute_enables_tests(meta: &Meta) -> bool {
    match (meta.path.as_str(), &meta.list) {
        ("cfg", Some(list)) => list.iter().any(condition_enables_tests),
        ("cfg_attr", Some(list)) => list.iter().skip(1).any(meta_attribute_enables_tests),
        _ => false,
    }
}

fn condition_enables_tests(meta: &Meta) -> bool {
    match (meta.path.as_str(), &meta.list) {
        ("test", None) => true,
        ("not", Some(_)) => false,
        (_, Some(list)) => list.iter().any(condition_enables_tests),
        _ => false,
    }
}

/// Split attribute text into identifiers (with `::` paths joined), string literals and the
/// punctuation `(`, `)`, `,` and `=`.
fn tokenize(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '"' {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += if chars[i] == '\\' { 2 } else { 1 };
            }
            i = (i + 1).min(chars.len());
            tokens.push(chars[start..i].iter().collect());
        } else if c.is_alphanumeric() || c == '_' || c == ':' {
            let start = i;
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == ':')
            {
                i += 1;
            }
            tokens.push(chars[start..i].iter().collect());
        } else {
            tokens.push(c.to_string());
            i += 1;
        }
    }
    tokens
}

/// `path`, `path = literal` or `path(meta, …)`; `None` when the tokens are not a meta item.
fn parse_meta(tokens: &[String], cursor: &mut usize) -> Option<Meta> {
    let path = tokens.get(*cursor)?.clone();
    if path.starts_with('"') || (path.len() == 1 && !path.chars().all(char::is_alphanumeric)) {
        return None;
    }
    *cursor += 1;
    match tokens.get(*cursor).map(String::as_str) {
        Some("=") => {
            *cursor += 2;
            Some(Meta { path, list: None })
        }
        Some("(") => {
            *cursor += 1;
            let mut list = Vec::new();
            while let Some(token) = tokens.get(*cursor) {
                match token.as_str() {
                    ")" => {
                        *cursor += 1;
                        return Some(Meta {
                            path,
                            list: Some(list),
                        });
                    }
                    "," => *cursor += 1,
                    _ => list.push(parse_meta(tokens, cursor)?),
                }
            }
            None
        }
        _ => Some(Meta { path, list: None }),
    }
}

#[cfg(test)]
#[path = "tests/sibling_test_placement.rs"]
mod tests;
