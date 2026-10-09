//! Literal-aware scanning of comment-stripped Rust source for the route table parser.
//!
//! **Role:** finds file-scope function bodies, balanced bracket groups, top-level comma splits,
//! method-call chains and string literal values in Rust source text, and maps a module file to
//! its child-module directory, so [`super::route_table`] can read router registrations without
//! a Rust parser.
//!
//! **Position:** test support under `tests/route_acceptance_support/`; reads source text that
//! [`read_stripped_source`] loads through the shared comment stripper in `tests/common`, and
//! feeds [`super::route_table`].
//!
//! **Signals & state:** none; pure functions over `&str`.
//!
//! **Invariants:** every scan skips string, raw string and char literals through
//! `common::source_text::rust_literal_end`, so a bracket or keyword inside a literal never
//! counts; a shape the scanner cannot read is an `Err` naming the text, never a silent skip.

use std::path::{Path, PathBuf};

use crate::common::source_text::{rust_literal_end, strip_rust_comments_outside_literals};

/// One call in a method chain: the callee path as written and the text between its parentheses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    /// The callee as written, such as `get`, `axum::routing::put` or `handlers::x::routes`.
    pub path: String,
    /// The argument text, without the enclosing parentheses.
    pub args: String,
}

impl Call {
    /// The last path segment: the function or method name.
    pub(crate) fn name(&self) -> &str {
        self.path.rsplit("::").next().unwrap_or(&self.path)
    }
}

/// The file's text with every comment removed and every literal kept.
pub(crate) fn read_stripped_source(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map(|text| strip_rust_comments_outside_literals(&text))
        .map_err(|error| format!("read {}: {error}", path.display()))
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Index of the bracket closing the one at `open`, skipping literals.
pub(crate) fn matching_close(text: &str, open: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Every index where `token` starts outside a literal, from `from` on.
pub(crate) fn find_outside_literals(text: &str, from: usize, token: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        if bytes[i..].starts_with(token.as_bytes()) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The body (between the braces, exclusive) of the single file-scope `fn name`, or `None` when
/// the file declares no such function at depth 0.
pub(crate) fn file_scope_fn_body<'a>(code: &'a str, name: &str) -> Result<Option<&'a str>, String> {
    let marker = format!("fn {name}");
    let bytes = code.as_bytes();
    let mut depth = 0i32;
    let mut starts = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ if depth == 0
                && bytes[i..].starts_with(marker.as_bytes())
                && (i == 0 || !is_ident_byte(bytes[i - 1]))
                && matches!(bytes.get(i + marker.len()), Some(b'(' | b'<')) =>
            {
                starts.push(i);
            }
            _ => {}
        }
        i += 1;
    }
    let start = match starts.as_slice() {
        [] => return Ok(None),
        [only] => *only,
        many => return Err(format!("{} file-scope `{marker}` definitions", many.len())),
    };
    let open =
        find_outside_literals(code, start, "{").ok_or_else(|| format!("`{marker}` has no body"))?;
    let close = matching_close(code, open).ok_or_else(|| format!("`{marker}` body not closed"))?;
    Ok(Some(&code[open + 1..close]))
}

/// `text` split at the depth-0 occurrences of `separator`, each piece trimmed; empty trailing
/// pieces (a trailing comma) are dropped.
pub(crate) fn split_top_level(text: &str, separator: u8) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut pieces = Vec::new();
    let mut depth = 0i32;
    let mut piece_start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if let Some(end) = rust_literal_end(bytes, i) {
            i = end;
            continue;
        }
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            byte if byte == separator && depth == 0 => {
                pieces.push(text[piece_start..i].trim());
                piece_start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    pieces.push(text[piece_start..].trim());
    while pieces.last().is_some_and(|piece| piece.is_empty()) {
        pieces.pop();
    }
    pieces
}

/// The value of a plain `"…"` literal (no escapes), or `None` for any other expression.
pub(crate) fn string_literal_value(expression: &str) -> Option<String> {
    let inner = expression.trim().strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains(['"', '\\'])).then(|| inner.to_string())
}

/// The depth-0 call chain `a::b(args).c(args)…` that makes up `expression`.
///
/// A generic turbofish or a field access in the chain is refused: route registrations never
/// use one, and a shape this reader does not understand must fail rather than be skipped.
pub(crate) fn call_chain(expression: &str) -> Result<Vec<Call>, String> {
    let text = expression.trim();
    let bytes = text.as_bytes();
    let mut calls = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'.') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let path_start = i;
        while i < bytes.len() && (is_ident_byte(bytes[i]) || bytes[i] == b':') {
            i += 1;
        }
        let path = text[path_start..i].to_string();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if path.is_empty() || bytes.get(i) != Some(&b'(') {
            return Err(format!("not a call chain at `{}`", &text[path_start..]));
        }
        let close = matching_close(text, i).ok_or_else(|| format!("unclosed call `{path}(`"))?;
        calls.push(Call {
            path,
            args: text[i + 1..close].to_string(),
        });
        i = close + 1;
    }
    Ok(calls)
}

/// True when `text[at..]` starts the keyword `word` (identifier boundaries on both sides).
pub(crate) fn keyword_at(text: &str, at: usize, word: &str) -> bool {
    let bytes = text.as_bytes();
    bytes[at..].starts_with(word.as_bytes())
        && (at == 0 || !is_ident_byte(bytes[at - 1]))
        && bytes
            .get(at + word.len())
            .is_none_or(|byte| !is_ident_byte(*byte))
}

/// The identifiers of `text`, in order (used to classify an `if` condition).
pub(crate) fn identifiers(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
        .collect()
}

/// The directory holding the child modules of the module in `file`.
pub(crate) fn module_directory(file: &Path) -> Option<PathBuf> {
    if file.file_name()? == "mod.rs" {
        return file.parent().map(Path::to_path_buf);
    }
    Some(file.with_extension(""))
}

/// The directory holding the child modules of `file`'s parent module (`super::`).
pub(crate) fn parent_dir_of_module(file: &Path) -> PathBuf {
    let own = module_directory(file).unwrap_or_default();
    own.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// Every `.rs` file under `dir`, recursively, sorted.
pub(crate) fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}
