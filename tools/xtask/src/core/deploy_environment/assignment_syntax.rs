//! The grammar of `deploy.env`: one `KEY=VALUE` assignment per line, read as data.
//!
//! **Role:** turns the file's text into its assignments, or names the first line that breaks the
//! grammar.
//!
//! **Position:** called by [`super::DeployEnvironment::from_text`]; the precedence between the
//! assignments and the process environment is decided there, not here.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** nothing is executed or expanded: `$(…)`, `$VAR` and backslashes stay literal
//! text. Blank lines and lines starting with `#` are skipped; an optional `export ` prefix is
//! dropped; the key matches `[A-Za-z_][A-Za-z0-9_]*`, with spaces allowed around `=`. A value in
//! `"…"` or `'…'` is taken verbatim and may be followed only by spaces or ` # comment`; an
//! unquoted value ends at the first `#` that follows whitespace and is trimmed, keeping its inner
//! spaces. Every error carries its 1-based line number and never echoes a value, which may be a
//! secret.

/// One assignment of the file, with the line it sits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Assignment {
    pub(super) key: String,
    pub(super) value: String,
    pub(super) line: usize,
}

/// Every assignment of `text` in file order, or the first `(line, message)` that breaks the
/// grammar. A repeated key appears once per assignment; the caller keeps the last.
pub(super) fn parse_assignments(text: &str) -> Result<Vec<Assignment>, (usize, String)> {
    let mut assignments = Vec::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (key, value) = parse_line(trimmed).map_err(|message| (line, message))?;
        assignments.push(Assignment { key, value, line });
    }
    Ok(assignments)
}

/// One non-blank, non-comment line.
fn parse_line(line: &str) -> Result<(String, String), String> {
    let body = match line.strip_prefix("export") {
        Some(rest) if rest.starts_with(char::is_whitespace) => rest.trim_start(),
        _ => line,
    };
    let Some((raw_key, raw_value)) = body.split_once('=') else {
        return Err("expected KEY=VALUE, and this line has no `=`".to_string());
    };
    let key = raw_key.trim();
    if key.is_empty() {
        return Err("the variable name before `=` is missing".to_string());
    }
    if !is_variable_name(key) {
        return Err(format!(
            "`{key}` is not a variable name: use letters, digits and `_`, not starting with a digit"
        ));
    }
    let value = match raw_value.trim_start().chars().next() {
        Some(quote @ ('"' | '\'')) => quoted_value(raw_value.trim_start(), quote)?,
        _ => unquoted_value(raw_value).to_string(),
    };
    Ok((key.to_string(), value))
}

fn is_variable_name(key: &str) -> bool {
    let mut characters = key.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_')
}

/// The text between `quote` at the start of `text` and its closing twin, verbatim.
fn quoted_value(text: &str, quote: char) -> Result<String, String> {
    let quote_name = if quote == '"' { "double" } else { "single" };
    let inner = &text[quote.len_utf8()..];
    let Some(close) = inner.find(quote) else {
        return Err(format!("unterminated {quote_name} quote"));
    };
    let after = &inner[close + quote.len_utf8()..];
    let ends_cleanly = after.trim().is_empty()
        || (after.starts_with(char::is_whitespace) && after.trim_start().starts_with('#'));
    if !ends_cleanly {
        return Err(format!(
            "text after the closing {quote_name} quote; only spaces or ` # comment` may follow it"
        ));
    }
    Ok(inner[..close].to_string())
}

/// The text after `=` up to the first `#` that follows whitespace, trimmed.
fn unquoted_value(text: &str) -> &str {
    let end = text
        .char_indices()
        .find(|&(index, character)| {
            character == '#' && text[..index].ends_with(char::is_whitespace)
        })
        .map_or(text.len(), |(index, _)| index);
    text[..end].trim()
}

#[cfg(test)]
#[path = "tests/assignment_syntax/tests.rs"]
mod tests;
