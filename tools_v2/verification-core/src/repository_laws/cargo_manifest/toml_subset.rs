//! The lexical helpers of the manifest reader: comments, strings, arrays and inline tables.
//!
//! **Role:** splits the TOML subset Cargo manifests are written in into the pieces
//! [`super::parse_manifest`] assembles: a line without its comment, whether an entry has closed,
//! the strings of an array, the value of one inline-table field.
//! **Position:** private to [`super`]; text in, text out.
//! **Signals & state:** none; pure functions.
//! **Invariants:** a `#` or bracket inside a `"…"` string is text, never syntax, so a comment
//! never hides an edge and a quoted bracket never opens an array.

/// `line` up to the first `#` that is not inside a string.
pub(super) fn without_comment(line: &str) -> &str {
    let mut quoted = false;
    for (offset, character) in line.char_indices() {
        match character {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..offset],
            _ => {}
        }
    }
    line
}

/// True when every `{` and `[` of `entry` outside strings has closed.
pub(super) fn balanced(entry: &str) -> bool {
    let mut depth = 0i32;
    let mut quoted = false;
    for character in entry.chars() {
        match character {
            '"' => quoted = !quoted,
            '{' | '[' if !quoted => depth += 1,
            '}' | ']' if !quoted => depth -= 1,
            _ => {}
        }
    }
    depth <= 0
}

/// `text` without surrounding quotes.
pub(super) fn unquoted(text: &str) -> String {
    text.trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

/// The first `"…"` string in `text`, or `text` itself without quotes.
pub(super) fn first_string(text: &str) -> String {
    text.split('"')
        .nth(1)
        .map(str::to_string)
        .unwrap_or_else(|| unquoted(text))
}

/// Every `"…"` string in `text`, in order — the items of an array or a single string value.
/// The first array closes the list, so `features = ["a"], optional = true` yields `a` only.
pub(super) fn string_items(text: &str) -> Vec<String> {
    let text = match (text.find('['), text.find(']')) {
        (Some(open), Some(close)) if open < close => &text[open..close],
        _ => text,
    };
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The text after `name =` inside an inline table, when `name` stands there as a whole key
/// (preceded by `{`, `,` or whitespace and followed by `=`).
pub(super) fn inline_field(value: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(found) = value[from..].find(name) {
        let start = from + found;
        let end = start + name.len();
        let before = value[..start].chars().next_back();
        let after = value[end..].trim_start();
        if matches!(before, Some('{' | ',' | ' ' | '\t'))
            && let Some(rest) = after.strip_prefix('=')
        {
            return Some(rest.trim_start().to_string());
        }
        from = end;
    }
    None
}

/// True when `value` is the TOML boolean `true` (the first token of the text).
pub(super) fn is_true(value: &str) -> bool {
    value.trim_start().starts_with("true")
}

/// True when `value` is an inline table that sets `workspace = true`.
pub(super) fn inherits_from_workspace(value: &str) -> bool {
    value.trim_start().starts_with('{')
        && inline_field(value, "workspace").is_some_and(|rest| is_true(&rest))
}
