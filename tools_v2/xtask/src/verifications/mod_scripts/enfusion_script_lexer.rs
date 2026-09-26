//! Lexes Enfusion script (`.c`) source into code, string literal and comment tokens.
//!
//! **Role:** the one place a mod-script check separates comments from code, so a `//` inside a
//! string literal is never read as a comment and a comment is never read as code.
//!
//! **Position:** fed raw `.c` text by the mod-script checks; [`strip_c_comments`] serves the source
//! pins of `destroy-target-diagnostics` and `mission-rest-size-limits`, [`split_script_lines`]
//! serves the per-line view of `enfusion-comments`.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:**
//! - Concatenating every [`ScriptToken::text`] reproduces the input exactly.
//! - A string literal opens at `"`, honours `\` escapes, and closes at the next unescaped `"` or,
//!   unterminated, before the end of its line, so one stray quote never swallows the file.
//! - A line comment excludes its terminating newline; a block comment keeps every newline it spans
//!   in the stripped text, so line numbers survive stripping.

/// What a [`ScriptToken`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// Code outside comments and string literals, newlines included.
    Code,
    /// A `"..."` literal, both quotes included.
    StringLiteral,
    /// A `//` comment up to, not including, its newline.
    LineComment,
    /// A `/* ... */` comment, delimiters included.
    BlockComment,
}

/// One lexed run of source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptToken {
    /// What the run is.
    pub(crate) kind: TokenKind,
    /// The run exactly as it appears in the source.
    pub(crate) text: String,
    /// The 1-based line the run starts on.
    pub(crate) line: usize,
}

/// The marker that opened a comment piece in a [`ScriptLine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommentMarker {
    /// `//`
    Plain,
    /// `//!`, a leading documentation line.
    Doc,
    /// `//!<`, a trailing documentation comment.
    TrailingDoc,
    /// One line of a `/* ... */` comment.
    Block,
}

/// The part of one comment that falls on one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineComment {
    /// The marker that opened the comment.
    pub(crate) marker: CommentMarker,
    /// The comment text without its marker, a block line's leading `*`, or surrounding blanks.
    pub(crate) text: String,
    /// Whether code precedes the comment on its line.
    pub(crate) after_code: bool,
}

/// One source line split into its code and its comments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ScriptLine {
    /// The line as written, without its line terminator.
    pub(crate) raw: String,
    /// The code of the line: comments removed, each string literal reduced to `""`.
    pub(crate) code: String,
    /// Every comment piece on the line, in source order.
    pub(crate) comments: Vec<LineComment>,
}

impl ScriptLine {
    /// Whether the line holds no code, only (or no) comments.
    pub(crate) fn is_comment_only(&self) -> bool {
        self.code.trim().is_empty()
    }
}

/// Splits `src` into [`ScriptToken`]s.
///
/// Returns the tokens in source order; never fails, since any byte sequence lexes.
pub(crate) fn tokenize_script(src: &str) -> Vec<ScriptToken> {
    let chars: Vec<char> = src.chars().collect();
    let mut tokens = Vec::new();
    let mut code = String::new();
    let mut code_line = 1;
    let mut line = 1;
    let mut index = 0;
    while index < chars.len() {
        if let Some((kind, end)) = token_end(&chars, index) {
            if !code.is_empty() {
                tokens.push(ScriptToken {
                    kind: TokenKind::Code,
                    text: std::mem::take(&mut code),
                    line: code_line,
                });
            }
            let text: String = chars[index..end].iter().collect();
            let spanned = text.matches('\n').count();
            tokens.push(ScriptToken { kind, text, line });
            line += spanned;
            index = end;
            continue;
        }
        if code.is_empty() {
            code_line = line;
        }
        code.push(chars[index]);
        if chars[index] == '\n' {
            line += 1;
        }
        index += 1;
    }
    if !code.is_empty() {
        tokens.push(ScriptToken {
            kind: TokenKind::Code,
            text: code,
            line: code_line,
        });
    }
    tokens
}

/// Drops `//` and `/* */` comments, keeping string literals verbatim and the newlines inside block
/// comments.
///
/// Returns the stripped source; never fails.
pub(crate) fn strip_c_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for token in tokenize_script(src) {
        match token.kind {
            TokenKind::Code | TokenKind::StringLiteral => out.push_str(&token.text),
            TokenKind::BlockComment => {
                out.extend(token.text.chars().filter(|c| *c == '\n'));
            }
            TokenKind::LineComment => {}
        }
    }
    out
}

/// Splits `src` into one [`ScriptLine`] per source line.
///
/// Returns as many lines as `src.lines()` yields, plus one when `src` ends without a newline
/// after a trailing empty line; never fails.
pub(crate) fn split_script_lines(src: &str) -> Vec<ScriptLine> {
    let mut lines: Vec<ScriptLine> = src
        .split('\n')
        .map(|raw| ScriptLine {
            raw: raw.strip_suffix('\r').unwrap_or(raw).to_string(),
            ..ScriptLine::default()
        })
        .collect();
    for token in tokenize_script(src) {
        let first = token.line - 1;
        match token.kind {
            TokenKind::Code => {
                let mut index = first;
                for c in token.text.chars() {
                    if c == '\n' {
                        index += 1;
                    } else if c != '\r' {
                        lines[index].code.push(c);
                    }
                }
            }
            TokenKind::StringLiteral => lines[first].code.push_str("\"\""),
            TokenKind::LineComment => {
                let after_code = !lines[first].is_comment_only();
                let (marker, text) = line_comment_parts(&token.text);
                lines[first].comments.push(LineComment {
                    marker,
                    text: text.trim().to_string(),
                    after_code,
                });
            }
            TokenKind::BlockComment => push_block_comment(&mut lines, first, &token.text),
        }
    }
    lines
}

/// The marker of a `//` comment and its text after the marker.
fn line_comment_parts(text: &str) -> (CommentMarker, &str) {
    if let Some(rest) = text.strip_prefix("//!<") {
        (CommentMarker::TrailingDoc, rest)
    } else if let Some(rest) = text.strip_prefix("//!") {
        (CommentMarker::Doc, rest)
    } else {
        (
            CommentMarker::Plain,
            text.strip_prefix("//").unwrap_or(text),
        )
    }
}

/// Records one block comment as a [`CommentMarker::Block`] piece on every line it spans.
fn push_block_comment(lines: &mut [ScriptLine], first: usize, text: &str) {
    let body = text.strip_prefix("/*").unwrap_or(text);
    let body = body.strip_suffix("*/").unwrap_or(body);
    for (offset, piece) in body.split('\n').enumerate() {
        let piece = piece.trim_end_matches('\r').trim();
        let piece = piece.strip_prefix('*').unwrap_or(piece).trim();
        let after_code = offset == 0 && !lines[first].is_comment_only();
        lines[first + offset].comments.push(LineComment {
            marker: CommentMarker::Block,
            text: piece.to_string(),
            after_code,
        });
    }
}

/// When a comment or string literal starts at `start`, its kind and exclusive end index.
fn token_end(chars: &[char], start: usize) -> Option<(TokenKind, usize)> {
    match (chars[start], chars.get(start + 1)) {
        ('/', Some('/')) => Some((TokenKind::LineComment, line_end(chars, start))),
        ('/', Some('*')) => Some((TokenKind::BlockComment, block_end(chars, start + 2))),
        ('"', _) => Some((TokenKind::StringLiteral, string_end(chars, start + 1))),
        _ => None,
    }
}

/// The index of the newline ending the line that holds `from`, or the input length.
fn line_end(chars: &[char], from: usize) -> usize {
    let mut index = from;
    while index < chars.len() && chars[index] != '\n' {
        index += 1;
    }
    index
}

/// The index just past the `*/` closing a block comment whose body starts at `from`.
fn block_end(chars: &[char], from: usize) -> usize {
    let mut index = from;
    while index + 1 < chars.len() {
        if chars[index] == '*' && chars[index + 1] == '/' {
            return index + 2;
        }
        index += 1;
    }
    chars.len()
}

/// The index just past the `"` closing a string literal whose body starts at `from`.
fn string_end(chars: &[char], from: usize) -> usize {
    let mut index = from;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 2,
            '"' => return index + 1,
            '\n' => return index,
            _ => index += 1,
        }
    }
    chars.len()
}
