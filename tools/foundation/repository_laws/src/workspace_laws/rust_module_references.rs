//! The module paths a Rust source file names inside its own crate.
//!
//! **Role:** finds every `crate::…` and `super::…` path in a source file — in `use` lines, in
//! braced `use` groups and inline in expressions and types — and resolves it to a module path
//! from the crate root.
//! **Position:** private to [`super`]; the input of [`super::frontend_layering`].
//! **Signals & state:** none; pure functions over source text.
//! **Invariants:** comments, string literals and character literals are blanked before the scan,
//! so a path named in a doc comment or a test's string never counts; `$crate::` (a macro's crate)
//! and a path segment after another `::` never start a reference; line numbers survive blanking.

/// One module path a source file names, resolved from the crate root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModuleReference {
    /// The module path's segments from the crate root, e.g. `["workspaces", "editor"]`.
    pub segments: Vec<String>,
    /// 1-based line where the path starts.
    pub line_no: usize,
}

/// The module of the source file at crate-relative `rel` (`src/foundation/ui/select.rs` →
/// `foundation::ui::select`); a `mod.rs`, `main.rs` or `lib.rs` is its folder's module and a
/// `tests` folder is transparent, so a sibling test file resolves `super` to the module it tests.
pub(super) fn file_module(rel: &str) -> Vec<String> {
    let rel = rel.strip_prefix("src/").unwrap_or(rel);
    let mut segments: Vec<String> = rel
        .trim_end_matches(".rs")
        .split('/')
        .filter(|part| *part != "tests")
        .map(str::to_string)
        .collect();
    let at_root = segments.len() == 1 && matches!(segments[0].as_str(), "main" | "lib");
    if at_root || segments.last().is_some_and(|last| last == "mod") {
        segments.pop();
    }
    segments
}

/// Every in-crate module path `text` names, resolved against `module` (the file's own module).
pub(super) fn module_references(text: &str, module: &[String]) -> Vec<ModuleReference> {
    let code = blank_comments_and_strings(text);
    let bytes = code.as_bytes();
    let mut references = Vec::new();
    for keyword in ["crate", "super"] {
        let needle = format!("{keyword}::");
        let mut from = 0;
        while let Some(found) = code[from..].find(&needle) {
            let start = from + found;
            from = start + needle.len();
            let before = start.checked_sub(1).map(|at| bytes[at]);
            if before
                .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$' || b == b':')
            {
                continue;
            }
            let line_no = code[..start].matches('\n').count() + 1;
            for path in expand_path(&code[start..]) {
                if let Some(segments) = resolve(&path, module) {
                    references.push(ModuleReference { segments, line_no });
                }
            }
        }
    }
    references.sort_by_key(|reference| reference.line_no);
    references
}

/// The paths a path expression starting at `text` spells: one path, or one per item of a braced
/// group (`a::{b::c, d}` → `a::b::c`, `a::d`).
fn expand_path(text: &str) -> Vec<Vec<String>> {
    let mut prefix: Vec<String> = Vec::new();
    let mut rest = text;
    loop {
        let identifier: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if identifier.is_empty() {
            break;
        }
        prefix.push(identifier.clone());
        rest = rest[identifier.len()..].trim_start();
        let Some(after) = rest.strip_prefix("::") else {
            break;
        };
        rest = after.trim_start();
        if let Some(group) = rest.strip_prefix('{') {
            return group_items(group)
                .into_iter()
                .flat_map(expand_path)
                .map(|tail| prefix.iter().cloned().chain(tail).collect())
                .collect();
        }
    }
    vec![prefix]
}

/// The top-level comma-separated items of a braced group whose opening `{` is already consumed.
fn group_items(group: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (offset, character) in group.char_indices() {
        match character {
            '{' => depth += 1,
            '}' if depth == 0 => {
                items.push(group[start..offset].trim());
                break;
            }
            '}' => depth -= 1,
            ',' if depth == 0 => {
                items.push(group[start..offset].trim());
                start = offset + 1;
            }
            _ => {}
        }
    }
    // A trailing comma leaves an empty item, which names nothing.
    items.retain(|item| !item.is_empty());
    items
}

/// `path` from the crate root: `crate::a` → `a`, `super::a` → the parent of `module` plus `a`.
fn resolve(path: &[String], module: &[String]) -> Option<Vec<String>> {
    let (first, rest) = path.split_first()?;
    if first == "crate" {
        return Some(rest.to_vec());
    }
    let mut base = module.to_vec();
    let mut tail = path;
    while let Some((head, after)) = tail.split_first() {
        if head != "super" {
            break;
        }
        base.pop()?;
        tail = after;
    }
    base.extend(tail.iter().cloned());
    Some(base)
}

/// `text` with every comment, string literal and character literal replaced by spaces, newlines
/// kept.
pub(super) fn blank_comments_and_strings(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    while index < chars.len() {
        let c = chars[index];
        let next = chars.get(index + 1).copied();
        let end = if c == '/' && next == Some('/') {
            chars[index..]
                .iter()
                .position(|c| *c == '\n')
                .map_or(chars.len(), |p| index + p)
        } else if c == '/' && next == Some('*') {
            (index + 2..chars.len().saturating_sub(1))
                .find(|at| chars[*at] == '*' && chars[*at + 1] == '/')
                .map_or(chars.len(), |at| at + 2)
        } else if c == 'r'
            && matches!(next, Some('"' | '#'))
            && !is_identifier_char(chars.get(index.wrapping_sub(1)))
        {
            raw_string_end(&chars, index).unwrap_or(index + 1)
        } else if c == '"' {
            quoted_end(&chars, index, '"')
        } else if c == '\'' && (chars.get(index + 2) == Some(&'\'') || next == Some('\\')) {
            quoted_end(&chars, index, '\'')
        } else {
            index + 1
        };
        if end == index + 1 && !matches!(c, '"' | '\'') {
            out.push(c);
        } else {
            out.extend(chars[index..end].iter().map(|c| blank(*c)));
        }
        index = end;
    }
    out
}

/// True when `c` is an identifier character.
fn is_identifier_char(c: Option<&char>) -> bool {
    c.is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')
}

/// The index just past the closing `quote` of the literal opening at `start`, skipping escapes.
fn quoted_end(chars: &[char], start: usize, quote: char) -> usize {
    let mut index = start + 1;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 2,
            c if c == quote => return index + 1,
            _ => index += 1,
        }
    }
    chars.len()
}

/// The index just past a raw string `r#…"…"#…` opening at `start`, when one opens there.
fn raw_string_end(chars: &[char], start: usize) -> Option<usize> {
    let hashes = chars[start + 1..].iter().take_while(|c| **c == '#').count();
    let quote = start + 1 + hashes;
    if chars.get(quote) != Some(&'"') {
        return None;
    }
    let mut index = quote + 1;
    while index < chars.len() {
        if chars[index] == '"'
            && chars[index + 1..]
                .iter()
                .take(hashes)
                .filter(|c| **c == '#')
                .count()
                == hashes
        {
            return Some(index + 1 + hashes);
        }
        index += 1;
    }
    Some(chars.len())
}
