//! A lexical scanner for Rust source: tokens with byte spans.
//!
//! **Role:** splits Rust source into identifiers, literals, punctuation and comments, so the
//! relocation passes can tell code from string literals and comments without a full parser and
//! without losing a byte of the original text.
//!
//! **Position:** read by the relative-reference pass ([`super::path_references`]) for string
//! literals and comments in `.rs` files, and by the Rust path pass ([`super::rust_paths`]) for
//! module declarations, `use` trees and path expressions; the plan builder and the verification
//! read [`code_spans`] to keep this tool's own test fixtures out of every rewrite.
//!
//! **Signals & state:** none; pure functions over one source string.
//!
//! **Invariants:** tokens are in source order, never overlap, and every byte outside a token is
//! whitespace; an unterminated literal or comment runs to the end of the source rather than
//! failing; raw strings (`r#"…"#`), byte and C strings, nested block comments, lifetimes and
//! character literals are told apart, so a quote inside one never opens another.

use std::ops::Range;

/// What one token is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// An identifier or keyword, raw identifiers (`r#type`) included.
    Identifier,
    /// A lifetime or loop label, `'a`.
    Lifetime,
    /// A string literal of any flavour: plain, byte, C, raw.
    StringLiteral,
    /// A character or byte-character literal.
    CharacterLiteral,
    /// A numeric literal.
    Number,
    /// One punctuation character.
    Punctuation,
    /// A `//` comment, doc comments included, without its line break.
    LineComment,
    /// A `/* … */` comment, nested comments included.
    BlockComment,
}

/// One token: its kind and its byte span in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Token {
    /// What the token is.
    pub(crate) kind: TokenKind,
    /// First byte of the token.
    pub(crate) start: usize,
    /// One past the last byte of the token.
    pub(crate) end: usize,
}

impl Token {
    /// The token's text.
    pub(crate) fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }

    /// Whether the token is a comment of either form.
    pub(crate) fn is_comment(&self) -> bool {
        matches!(self.kind, TokenKind::LineComment | TokenKind::BlockComment)
    }

    /// Whether the token is the punctuation character `character`.
    pub(crate) fn is_punctuation(&self, source: &str, character: char) -> bool {
        self.kind == TokenKind::Punctuation && source[self.start..].starts_with(character)
    }

    /// Whether the token is the identifier or keyword `word`.
    pub(crate) fn is_word(&self, source: &str, word: &str) -> bool {
        self.kind == TokenKind::Identifier && self.text(source) == word
    }
}

/// Every token of `source`, in order.
pub(crate) fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_whitespace() {
            at += 1;
            continue;
        }
        let (kind, end) = scan_token(source, at);
        tokens.push(Token {
            kind,
            start: at,
            end,
        });
        at = end;
    }
    tokens
}

/// The tokens that are not comments, in order.
pub(crate) fn code_tokens(tokens: &[Token]) -> Vec<Token> {
    tokens.iter().copied().filter(|t| !t.is_comment()).collect()
}

/// The spans of `source` outside every string literal and comment, in source order: the code a
/// file's fixture text never reaches.
pub(crate) fn code_spans(source: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut cursor = 0;
    for token in tokenize(source) {
        if token.kind == TokenKind::StringLiteral || token.is_comment() {
            if cursor < token.start {
                spans.push(cursor..token.start);
            }
            cursor = token.end;
        }
    }
    if cursor < source.len() {
        spans.push(cursor..source.len());
    }
    spans
}

/// The span of a string literal's content: the bytes between its opening and closing quote.
pub(crate) fn string_content(source: &str, token: &Token) -> Range<usize> {
    let text = token.text(source);
    let quote = text.find('"').unwrap_or(0);
    let hashes = text[..quote].bytes().filter(|b| *b == b'#').count();
    let open = quote + 1;
    let terminator_length = 1 + hashes;
    let terminated = text.len() >= open + terminator_length
        && text.as_bytes()[text.len() - terminator_length] == b'"'
        && text[text.len() - hashes..].bytes().all(|b| b == b'#');
    let close = if terminated {
        text.len() - terminator_length
    } else {
        text.len()
    };
    token.start + open..token.start + close
}

/// The 1-based line number of byte `offset` in `source`.
pub(crate) fn line_of(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        + 1
}

fn scan_token(source: &str, at: usize) -> (TokenKind, usize) {
    let bytes = source.as_bytes();
    let byte = bytes[at];
    let next = bytes.get(at + 1).copied();
    if byte == b'/' && next == Some(b'/') {
        let end = source[at..].find('\n').map_or(source.len(), |n| at + n);
        return (TokenKind::LineComment, end);
    }
    if byte == b'/' && next == Some(b'*') {
        return (TokenKind::BlockComment, block_comment_end(bytes, at));
    }
    if let Some(end) = prefixed_literal_end(source, at) {
        return end;
    }
    if byte == b'"' {
        return (TokenKind::StringLiteral, quoted_end(bytes, at + 1, b'"'));
    }
    if byte == b'\'' {
        return quote_token(source, at);
    }
    if byte.is_ascii_digit() {
        return (TokenKind::Number, number_end(bytes, at));
    }
    if is_identifier_start(source, at) {
        return (TokenKind::Identifier, identifier_end(source, at));
    }
    let width = source[at..].chars().next().map_or(1, char::len_utf8);
    (TokenKind::Punctuation, at + width)
}

/// A literal that starts with a letter prefix: raw identifiers, raw strings, byte and C strings
/// and byte characters.
fn prefixed_literal_end(source: &str, at: usize) -> Option<(TokenKind, usize)> {
    let bytes = source.as_bytes();
    let rest = &bytes[at..];
    let prefix_length = if rest.starts_with(b"br") || rest.starts_with(b"cr") {
        2
    } else if rest.starts_with(b"r") {
        1
    } else {
        0
    };
    if prefix_length > 0 {
        let after = &rest[prefix_length..];
        let hashes = after.iter().take_while(|b| **b == b'#').count();
        if after.get(hashes) == Some(&b'"') {
            let body = at + prefix_length + hashes + 1;
            return Some((
                TokenKind::StringLiteral,
                raw_string_end(bytes, body, hashes),
            ));
        }
        if prefix_length == 1 && hashes == 1 && is_identifier_start(source, at + 2) {
            return Some((TokenKind::Identifier, identifier_end(source, at + 2)));
        }
    }
    if (rest.starts_with(b"b\"") || rest.starts_with(b"c\"")) && !is_continuation(source, at) {
        return Some((TokenKind::StringLiteral, quoted_end(bytes, at + 2, b'"')));
    }
    if rest.starts_with(b"b'") && !is_continuation(source, at) {
        return Some((
            TokenKind::CharacterLiteral,
            quoted_end(bytes, at + 2, b'\''),
        ));
    }
    None
}

/// Whether the byte before `at` continues an identifier, which makes `at` no literal prefix.
fn is_continuation(source: &str, at: usize) -> bool {
    source[..at]
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_')
}

fn raw_string_end(bytes: &[u8], body: usize, hashes: usize) -> usize {
    let mut at = body;
    while at < bytes.len() {
        if bytes[at] == b'"' && bytes[at + 1..].iter().take(hashes).all(|b| *b == b'#') {
            let end = at + 1 + hashes;
            if end <= bytes.len() {
                return end;
            }
        }
        at += 1;
    }
    bytes.len()
}

/// The end of a quoted literal whose body starts at `body`, honouring backslash escapes.
fn quoted_end(bytes: &[u8], body: usize, quote: u8) -> usize {
    let mut at = body;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b if b == quote => return at + 1,
            _ => at += 1,
        }
    }
    bytes.len()
}

/// A character literal (`'a'`, `'\n'`, `'é'`) or a lifetime (`'a`, `'static`).
fn quote_token(source: &str, at: usize) -> (TokenKind, usize) {
    let bytes = source.as_bytes();
    if bytes.get(at + 1) == Some(&b'\\') {
        return (
            TokenKind::CharacterLiteral,
            quoted_end(bytes, at + 1, b'\''),
        );
    }
    let mut characters = source[at + 1..].char_indices();
    if let Some((_, first)) = characters.next() {
        let after_first = at + 1 + first.len_utf8();
        if bytes.get(after_first) == Some(&b'\'') {
            return (TokenKind::CharacterLiteral, after_first + 1);
        }
        if first.is_alphanumeric() || first == '_' {
            return (TokenKind::Lifetime, identifier_end(source, at + 1));
        }
    }
    (TokenKind::Punctuation, at + 1)
}

fn block_comment_end(bytes: &[u8], at: usize) -> usize {
    let mut depth = 0usize;
    let mut index = at;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && bytes[index + 1] == b'*' {
            depth += 1;
            index += 2;
        } else if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn number_end(bytes: &[u8], at: usize) -> usize {
    let mut index = at;
    while index < bytes.len() {
        let byte = bytes[index];
        let fraction = byte == b'.'
            && bytes.get(index + 1).is_some_and(u8::is_ascii_digit)
            && !bytes[at..index].contains(&b'.');
        if byte.is_ascii_alphanumeric() || byte == b'_' || fraction {
            index += 1;
        } else {
            break;
        }
    }
    index
}

fn is_identifier_start(source: &str, at: usize) -> bool {
    source[at..]
        .chars()
        .next()
        .is_some_and(|c| c == '_' || c.is_alphabetic())
}

fn identifier_end(source: &str, at: usize) -> usize {
    source[at..]
        .char_indices()
        .find(|(_, c)| !(c.is_alphanumeric() || *c == '_'))
        .map_or(source.len(), |(offset, _)| at + offset)
}
