//! The relative path literals of one file, each with the anchors it is read against.
//!
//! **Role:** finds the [`RelativeCandidate`]s of a file by its kind. In Rust source: the argument
//! of `include!`, `include_str!` and `include_bytes!` and of `#[path = "…"]` (read from the file's
//! folder, or from the crate folder when `CARGO_MANIFEST_DIR` builds the path), and every path
//! token inside other string literals and comments. In Markdown: every link destination (read
//! from the document's folder) and every `./` or `../` token elsewhere. In a Cargo manifest: every
//! `path = "…"`. In any other text file: every `./` or `../` token.
//!
//! **Position:** called by [`super`] for each file the path rows rewrite; each candidate goes to
//! [`super::anchor_resolution`].
//!
//! **Signals & state:** none; pure functions over one file's text.
//!
//! **Invariants:** candidates never overlap; Rust code outside literals and comments yields none;
//! a URL, a fragment-only link and an absolute path are never candidates; a `/`-led token inside a
//! Rust literal is a candidate only in a statement that names `CARGO_MANIFEST_DIR`, where it is
//! read from the crate folder.

use std::ops::Range;

use super::anchor_resolution::AnchorKind;
use super::markdown_links::link_destinations;
use super::path_tokens::{is_segment_byte, token_end};
use crate::commands::refactor::relocate::rust_lexer::{
    Token, TokenKind, code_tokens, string_content, tokenize,
};

/// The anchors of a token led by `./` or `../` with no other context.
const ANY_ANCHOR: &[AnchorKind] = &[
    AnchorKind::FileFolder,
    AnchorKind::CrateFolder,
    AnchorKind::RepositoryRoot,
    AnchorKind::EveryCrateFolder,
];

/// The anchors of a path that only its own file can mean: a link, an `include!`, a `#[path]`.
const FILE_FOLDER: &[AnchorKind] = &[AnchorKind::FileFolder];

/// The anchor of a path built on `CARGO_MANIFEST_DIR`.
const CRATE_FOLDER: &[AnchorKind] = &[AnchorKind::CrateFolder];

/// The anchors of a plain path inside a Rust string literal; the repository root belongs to the
/// repository-root spelling pass.
const FILE_OR_CRATE_FOLDER: &[AnchorKind] = &[AnchorKind::FileFolder, AnchorKind::CrateFolder];

/// The macros whose string argument is a path read at compile time from the file's folder.
const INCLUDE_MACROS: [&str; 3] = ["include", "include_str", "include_bytes"];

/// The environment variable that names the crate folder.
const MANIFEST_FOLDER_VARIABLE: &str = "CARGO_MANIFEST_DIR";

/// One relative literal: the bytes to rewrite and how to read them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RelativeCandidate {
    /// The literal's bytes, without a leading `/` that joins it to a base.
    pub(crate) span: Range<usize>,
    /// Whether a `/` before the span joins the literal to its anchor (`CARGO_MANIFEST_DIR/…`).
    pub(crate) leading_slash: bool,
    /// The anchors to read the literal under, in order.
    pub(crate) anchors: &'static [AnchorKind],
}

/// The kind of text file, as far as relative references go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReferenceFileKind {
    Rust,
    Markdown,
    CargoManifest,
    Other,
}

impl ReferenceFileKind {
    /// The kind of the file at `path`.
    pub(crate) fn of(path: &str) -> ReferenceFileKind {
        let name = path.rsplit('/').next().unwrap_or(path);
        if name == "Cargo.toml" {
            ReferenceFileKind::CargoManifest
        } else if name.ends_with(".rs") {
            ReferenceFileKind::Rust
        } else if super::super::file_treatment::is_markdown(name) {
            ReferenceFileKind::Markdown
        } else {
            ReferenceFileKind::Other
        }
    }
}

/// Every relative candidate of `source`, a file of `kind`; `links_only` keeps the Markdown link
/// destinations alone.
pub(crate) fn relative_candidates(
    source: &str,
    kind: ReferenceFileKind,
    links_only: bool,
) -> Vec<RelativeCandidate> {
    let mut found = match kind {
        ReferenceFileKind::Rust => rust_candidates(source),
        ReferenceFileKind::Markdown => markdown_candidates(source, links_only),
        ReferenceFileKind::CargoManifest => {
            let mut manifest = cargo_path_values(source);
            manifest.extend(dot_led_tokens(source, 0..source.len(), ANY_ANCHOR));
            manifest
        }
        ReferenceFileKind::Other => dot_led_tokens(source, 0..source.len(), ANY_ANCHOR),
    };
    found.sort_by_key(|candidate| candidate.span.start);
    found.dedup_by(|later, earlier| later.span.start < earlier.span.end);
    found
}

fn markdown_candidates(source: &str, links_only: bool) -> Vec<RelativeCandidate> {
    let links = link_destinations(source);
    let mut found: Vec<RelativeCandidate> = links
        .iter()
        .filter_map(|span| link_path(source, span.clone()))
        .collect();
    if !links_only {
        found.extend(
            dot_led_tokens(source, 0..source.len(), ANY_ANCHOR)
                .into_iter()
                .filter(|token| {
                    !links
                        .iter()
                        .any(|link| link.start <= token.span.start && token.span.start < link.end)
                }),
        );
    }
    found
}

/// The path part of a link destination: no URL, no root link, no bare fragment.
fn link_path(source: &str, span: Range<usize>) -> Option<RelativeCandidate> {
    let destination = &source[span.clone()];
    let path_length = destination.find(['#', '?']).unwrap_or(destination.len());
    let path = &destination[..path_length];
    let is_url = path.contains("://") || path.starts_with("mailto:") || path.contains(':');
    if path.is_empty() || path.starts_with('/') || is_url {
        return None;
    }
    Some(RelativeCandidate {
        span: span.start..span.start + path_length,
        leading_slash: false,
        anchors: FILE_FOLDER,
    })
}

/// The values of `path = "…"` keys in a Cargo manifest.
fn cargo_path_values(source: &str) -> Vec<RelativeCandidate> {
    let mut found = Vec::new();
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        let mut rest = line;
        let mut base = offset;
        while let Some(at) = find_path_key(rest) {
            let after = &rest[at..];
            let Some(open) = after.find('"') else { break };
            let Some(length) = after[open + 1..].find('"') else {
                break;
            };
            let start = base + at + open + 1;
            found.push(RelativeCandidate {
                span: start..start + length,
                leading_slash: false,
                anchors: FILE_FOLDER,
            });
            let consumed = at + open + 1 + length + 1;
            rest = &rest[consumed..];
            base += consumed;
        }
        offset += line.len();
    }
    found
}

/// The offset of the next `path` key (`path = "`) in `text`, on a word boundary.
fn find_path_key(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find("path") {
        let start = from + at;
        let before_ok = start == 0
            || !(bytes[start - 1].is_ascii_alphanumeric()
                || bytes[start - 1] == b'_'
                || bytes[start - 1] == b'-');
        let rest = text[start + 4..].trim_start();
        if before_ok && rest.starts_with('=') && rest[1..].trim_start().starts_with('"') {
            return Some(start);
        }
        from = start + 4;
    }
    None
}

/// Every token in `range` led by `./` or `../` at a token boundary.
fn dot_led_tokens(
    source: &str,
    range: Range<usize>,
    anchors: &'static [AnchorKind],
) -> Vec<RelativeCandidate> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut index = range.start;
    while index < range.end {
        let opens = bytes[index] == b'.'
            && (index == 0 || !(is_segment_byte(bytes[index - 1]) || bytes[index - 1] == b'/'))
            && (source[index..].starts_with("./") || source[index..].starts_with("../"));
        if opens {
            let end = token_end(bytes, index).min(range.end);
            found.push(RelativeCandidate {
                span: index..end,
                leading_slash: false,
                anchors,
            });
            index = end.max(index + 1);
        } else {
            index += 1;
        }
    }
    found
}

fn rust_candidates(source: &str) -> Vec<RelativeCandidate> {
    let tokens = tokenize(source);
    let code = code_tokens(&tokens);
    let mut found = Vec::new();
    for token in &tokens {
        if token.is_comment() {
            found.extend(dot_led_tokens(source, token.start..token.end, ANY_ANCHOR));
        }
    }
    for (index, token) in code.iter().enumerate() {
        if token.kind != TokenKind::StringLiteral {
            continue;
        }
        let content = string_content(source, token);
        if content.is_empty() {
            continue;
        }
        let manifest_folder = statement_names_manifest_folder(source, &code, index);
        if is_path_attribute_value(source, &code, index) {
            found.push(whole_literal(content, FILE_FOLDER));
        } else if let Some(include) = include_call(source, &code, index) {
            found.extend(include_candidate(source, content, include));
        } else {
            found.extend(literal_tokens(source, content, manifest_folder));
        }
    }
    found
}

fn whole_literal(content: Range<usize>, anchors: &'static [AnchorKind]) -> RelativeCandidate {
    RelativeCandidate {
        span: content,
        leading_slash: false,
        anchors,
    }
}

/// Whether the literal at `code[index]` is the value of `#[path = "…"]`.
fn is_path_attribute_value(source: &str, code: &[Token], index: usize) -> bool {
    index >= 3
        && code[index - 1].is_punctuation(source, '=')
        && code[index - 2].is_word(source, "path")
        && code[index - 3].is_punctuation(source, '[')
}

/// How an `include*!` call reads its path: `Some(true)` when `CARGO_MANIFEST_DIR` builds it.
fn include_call(source: &str, code: &[Token], index: usize) -> Option<bool> {
    let mut depth = 0usize;
    let mut manifest_folder = false;
    for back in (0..index).rev() {
        let token = &code[back];
        if token.is_punctuation(source, ')') {
            depth += 1;
        } else if token.is_punctuation(source, '(') {
            if depth > 0 {
                depth -= 1;
                continue;
            }
            let name = macro_name(source, code, back)?;
            if INCLUDE_MACROS.contains(&name) {
                return Some(manifest_folder);
            }
            if name != "concat" {
                return None;
            }
        } else if names_manifest_folder(source, token) {
            manifest_folder = true;
        } else if token.is_punctuation(source, ';') || token.is_punctuation(source, '{') {
            return None;
        }
    }
    None
}

/// The macro name before the `(` at `code[open]`, as in `name!(`.
fn macro_name<'a>(source: &'a str, code: &[Token], open: usize) -> Option<&'a str> {
    (open >= 2 && code[open - 1].is_punctuation(source, '!'))
        .then(|| code[open - 2])
        .filter(|token| token.kind == TokenKind::Identifier)
        .map(|token| token.text(source))
}

fn include_candidate(
    source: &str,
    content: Range<usize>,
    manifest_folder: bool,
) -> Option<RelativeCandidate> {
    if !manifest_folder {
        return Some(whole_literal(content, FILE_FOLDER));
    }
    source[content.clone()]
        .starts_with('/')
        .then(|| RelativeCandidate {
            span: content.start + 1..content.end,
            leading_slash: true,
            anchors: CRATE_FOLDER,
        })
}

/// The path tokens inside one plain string literal.
fn literal_tokens(
    source: &str,
    content: Range<usize>,
    manifest_folder: bool,
) -> Vec<RelativeCandidate> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut index = content.start;
    while index < content.end {
        let at_boundary = index == content.start
            || !(is_segment_byte(bytes[index - 1]) || bytes[index - 1] == b'/');
        if !at_boundary || !(is_segment_byte(bytes[index]) || bytes[index] == b'/') {
            index += 1;
            continue;
        }
        let end = token_end(bytes, index).min(content.end);
        let token = &source[index..end];
        let slash_led = token.starts_with('/') && token.len() > 1;
        let dot_led = token.starts_with("./") || token.starts_with("../");
        let candidate = if slash_led && manifest_folder {
            Some(RelativeCandidate {
                span: index + 1..end,
                leading_slash: true,
                anchors: CRATE_FOLDER,
            })
        } else if dot_led {
            let anchors = if manifest_folder {
                CRATE_FOLDER
            } else {
                ANY_ANCHOR
            };
            Some(whole_literal(index..end, anchors))
        } else if !slash_led && token.contains('/') {
            let anchors = if manifest_folder {
                CRATE_FOLDER
            } else {
                FILE_OR_CRATE_FOLDER
            };
            Some(whole_literal(index..end, anchors))
        } else {
            None
        };
        found.extend(candidate);
        index = end.max(index + 1);
    }
    found
}

/// Whether the statement holding `code[index]` names `CARGO_MANIFEST_DIR`.
fn statement_names_manifest_folder(source: &str, code: &[Token], index: usize) -> bool {
    let is_boundary = |token: &Token| {
        token.is_punctuation(source, ';')
            || token.is_punctuation(source, '{')
            || token.is_punctuation(source, '}')
    };
    let start = (0..index)
        .rev()
        .find(|at| is_boundary(&code[*at]))
        .map_or(0, |at| at + 1);
    let end = (index..code.len())
        .find(|at| is_boundary(&code[*at]))
        .unwrap_or(code.len());
    code[start..end]
        .iter()
        .any(|token| names_manifest_folder(source, token))
}

fn names_manifest_folder(source: &str, token: &Token) -> bool {
    match token.kind {
        TokenKind::Identifier => token.text(source) == MANIFEST_FOLDER_VARIABLE,
        TokenKind::StringLiteral => {
            &source[string_content(source, token)] == MANIFEST_FOLDER_VARIABLE
        }
        _ => false,
    }
}
