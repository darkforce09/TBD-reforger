//! The relocation manifest: a tab-separated list of moves and rewrites.
//!
//! **Role:** parses and validates one manifest file into [`ManifestRow`]s: `path` rows that move a
//! tracked file or folder, `rust_path` rows that rewrite a Rust path prefix, and `text` rows that
//! rewrite an identifier-like token.
//!
//! **Position:** the first step of every `cargo xtask refactor relocate` mode; the plan builder
//! ([`super::relocation_plan`]) and the verification ([`super::retired_spellings`]) read the rows.
//!
//! **Signals & state:** none; pure functions over the manifest text.
//!
//! **Invariants:** blank lines and lines starting with `#` are skipped; the first other line is the
//! header `kind<TAB>from<TAB>to<TAB>scope`; every row has three or four columns; a manifest with
//! any invalid row is refused as a whole, every error named with its line, so a half-valid
//! manifest never runs.

/// The header line every manifest starts with, after its comments.
pub(crate) const MANIFEST_HEADER: &str = "kind\tfrom\tto\tscope";

/// What a row asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowKind {
    /// Move a tracked file or folder and rewrite every reference to it.
    Path,
    /// Rewrite a Rust path prefix, such as `crate::a::b::`.
    RustPath,
    /// Rewrite an identifier-like token on word boundaries.
    Text,
}

impl RowKind {
    /// The kind's spelling in the manifest's first column.
    pub(crate) fn label(self) -> &'static str {
        match self {
            RowKind::Path => "path",
            RowKind::RustPath => "rust_path",
            RowKind::Text => "text",
        }
    }
}

/// Which files a `rust_path` or `text` row rewrites, as paths before any move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RowScope {
    /// Every candidate file.
    Everywhere,
    /// The files at or below one repository folder.
    Folder(String),
    /// The files whose repository path matches a glob (`*`, `**`, `?`).
    Glob(String),
}

impl RowScope {
    /// Whether `path` (repository-relative, `/`-separated) is in the scope.
    pub(crate) fn contains(&self, path: &str) -> bool {
        match self {
            RowScope::Everywhere => true,
            RowScope::Folder(folder) => super::path_mapping::is_at_or_below(path, folder),
            RowScope::Glob(pattern) => glob_matches(pattern.as_bytes(), path.as_bytes()),
        }
    }

    /// The scope as the manifest spells it; empty for [`RowScope::Everywhere`].
    pub(crate) fn spelling(&self) -> &str {
        match self {
            RowScope::Everywhere => "",
            RowScope::Folder(text) | RowScope::Glob(text) => text,
        }
    }
}

/// One validated manifest row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ManifestRow {
    /// The row's 1-based line in the manifest, its name in every report.
    pub(crate) line: usize,
    /// What the row asks for.
    pub(crate) kind: RowKind,
    /// The retired spelling.
    pub(crate) from: String,
    /// The spelling that replaces it.
    pub(crate) to: String,
    /// The files the row rewrites; always [`RowScope::Everywhere`] for a `path` row.
    pub(crate) scope: RowScope,
}

/// Parse `text` into rows, or every error found, each as `line N: message`.
pub(crate) fn parse_manifest(text: &str) -> Result<Vec<ManifestRow>, Vec<String>> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut header_seen = false;
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = raw.trim_end_matches('\r');
        if content.trim().is_empty() || content.trim_start().starts_with('#') {
            continue;
        }
        if !header_seen {
            header_seen = true;
            if content.trim_end() != MANIFEST_HEADER {
                errors.push(format!(
                    "line {line}: the first row must be the header `kind\\tfrom\\tto\\tscope`"
                ));
            }
            continue;
        }
        match parse_row(line, content) {
            Ok(row) => rows.push(row),
            Err(message) => errors.push(format!("line {line}: {message}")),
        }
    }
    if !header_seen {
        errors.push("the manifest has no header row".to_string());
    }
    errors.extend(duplicate_sources(&rows));
    if errors.is_empty() {
        Ok(rows)
    } else {
        Err(errors)
    }
}

fn parse_row(line: usize, content: &str) -> Result<ManifestRow, String> {
    let columns: Vec<&str> = content.split('\t').collect();
    if !(3..=4).contains(&columns.len()) {
        return Err(format!(
            "expected 3 or 4 tab-separated columns, found {}",
            columns.len()
        ));
    }
    let kind = match columns[0] {
        "path" => RowKind::Path,
        "rust_path" => RowKind::RustPath,
        "text" => RowKind::Text,
        other => return Err(format!("unknown kind `{other}` (path, rust_path or text)")),
    };
    let scope_text = columns.get(3).copied().unwrap_or("").trim();
    let (from, to) = match kind {
        RowKind::Path => (
            valid_repository_path(columns[1])?,
            valid_repository_path(columns[2])?,
        ),
        RowKind::RustPath => (
            valid_rust_prefix(columns[1])?,
            valid_rust_prefix(columns[2])?,
        ),
        RowKind::Text => (valid_token(columns[1])?, valid_token(columns[2])?),
    };
    if from == to {
        return Err("`from` and `to` are the same".to_string());
    }
    let scope = match kind {
        RowKind::Path if !scope_text.is_empty() => {
            return Err("a path row takes no scope".to_string());
        }
        _ if scope_text.is_empty() => RowScope::Everywhere,
        _ if scope_text.contains(['*', '?']) => RowScope::Glob(scope_text.to_string()),
        _ => RowScope::Folder(valid_repository_path(scope_text)?),
    };
    if kind == RowKind::Path && super::path_mapping::is_at_or_below(&to, &from) {
        return Err(format!(
            "`{to}` lies inside `{from}`, which cannot move into itself"
        ));
    }
    Ok(ManifestRow {
        line,
        kind,
        from,
        to,
        scope,
    })
}

/// A repository-relative path: no leading `/`, no `.` or `..` segment, no empty segment, no
/// whitespace; a trailing `/` is dropped.
fn valid_repository_path(text: &str) -> Result<String, String> {
    let path = text.trim().trim_end_matches('/');
    let bad_segment = path
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..");
    if path.is_empty() || path.starts_with('/') || bad_segment {
        return Err(format!("`{text}` is not a repository-relative path"));
    }
    if path.chars().any(|c| c.is_whitespace() || c == '\\') {
        return Err(format!("`{text}` holds whitespace or a backslash"));
    }
    Ok(path.to_string())
}

/// A Rust path prefix: identifier segments joined and ended by `::`, such as `crate::a::`.
fn valid_rust_prefix(text: &str) -> Result<String, String> {
    let prefix = text.trim();
    let Some(body) = prefix.strip_suffix("::") else {
        return Err(format!("`{text}` must end in `::`"));
    };
    let valid = body.split("::").all(|segment| {
        !segment.is_empty()
            && segment.chars().all(|c| c.is_alphanumeric() || c == '_')
            && !segment.starts_with(|c: char| c.is_ascii_digit())
    });
    if !valid {
        return Err(format!("`{text}` is not a Rust path prefix"));
    }
    Ok(prefix.to_string())
}

/// An identifier-like token: letters, digits, `_`, `-` and `.`, starting with a letter, digit or
/// `_`.
fn valid_token(text: &str) -> Result<String, String> {
    let token = text.trim();
    let valid = token
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
        && token
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if !valid {
        return Err(format!("`{text}` is not an identifier-like token"));
    }
    Ok(token.to_string())
}

/// Two rows of one kind may not retire the same spelling over overlapping files.
fn duplicate_sources(rows: &[ManifestRow]) -> Vec<String> {
    let mut errors = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        for earlier in &rows[..index] {
            if earlier.kind == row.kind && earlier.from == row.from && earlier.scope == row.scope {
                errors.push(format!(
                    "line {}: repeats the `from` of line {}",
                    row.line, earlier.line
                ));
            }
        }
    }
    errors
}

/// Whether `path` matches `pattern`: `**` spans folders, `*` and `?` stay inside one name.
pub(crate) fn glob_matches(pattern: &[u8], path: &[u8]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((b'*', rest)) if rest.first() == Some(&b'*') => {
            let after = rest[1..].strip_prefix(b"/").unwrap_or(&rest[1..]);
            (0..=path.len()).any(|skip| {
                (skip == 0 || path[skip - 1] == b'/' || after.is_empty())
                    && glob_matches(after, &path[skip..])
            })
        }
        Some((b'*', rest)) => (0..=path.len())
            .take_while(|skip| *skip == 0 || path[skip - 1] != b'/')
            .any(|skip| glob_matches(rest, &path[skip..])),
        Some((b'?', rest)) => path
            .split_first()
            .is_some_and(|(first, tail)| *first != b'/' && glob_matches(rest, tail)),
        Some((literal, rest)) => path
            .split_first()
            .is_some_and(|(first, tail)| first == literal && glob_matches(rest, tail)),
    }
}
