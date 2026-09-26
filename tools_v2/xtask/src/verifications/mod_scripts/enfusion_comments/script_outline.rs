//! Declaration outline of one Enfusion script: its types, methods, fields, enum members and the
//! attributes that decorate them.
//!
//! **Role:** turns the comment-free code of a script into the declarations the comment rules
//! judge, so every rule shares one deterministic reading of what is a method or a field.
//!
//! **Position:** fed the [`ScriptLine`]s of
//! [`crate::verifications::mod_scripts::enfusion_script_lexer::split_script_lines`]; consumed by
//! every rule module of `enfusion_comments` through [`ScriptOutline`].
//!
//! **Signals & state:** none; pure functions over one script.
//!
//! **Invariants:**
//! - A method is a class-body (or top-level) statement that holds `(` before any `=` and ends at
//!   `{` or `;`; a field is any other class-body statement ending at `;`.
//! - `[ ... ]` at the start of a member statement is an attribute of the member that follows.
//! - String literal contents never reach the outline, so braces, brackets and `;` inside strings
//!   cannot move a boundary.
//! - An unbalanced `}` at the top level is ignored rather than underflowing the scope stack.

use std::collections::BTreeSet;

use super::super::enfusion_script_lexer::ScriptLine;

/// What a [`ScriptItem`] declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ItemKind {
    /// A `class` or `modded class`.
    Class {
        /// Whether the declaration is `modded class`.
        modded: bool,
        /// The base class name after `:` or `extends`, when there is one.
        base: Option<String>,
    },
    /// An `enum`.
    Enum,
    /// A method, a top-level function or a body-less method declaration.
    Method,
    /// A class-body or top-level variable declaration.
    Field,
    /// One member of an enum.
    EnumMember,
}

/// One `[Name(...)]` attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptAttribute {
    /// The attribute name, such as `Attribute` or `RplRpc`.
    pub(crate) name: String,
    /// The attribute code, brackets included, string literal contents removed.
    pub(crate) text: String,
    /// The 1-based line of `[`.
    pub(crate) start_line: usize,
    /// The 1-based line of the closing `]`.
    pub(crate) end_line: usize,
}

/// One declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScriptItem {
    /// What the declaration is.
    pub(crate) kind: ItemKind,
    /// The declared name; empty when none could be read.
    pub(crate) name: String,
    /// The 1-based first line of the declaration itself, attributes excluded.
    pub(crate) start_line: usize,
    /// The 1-based line of the `{`, `;` or `,` that ends the declaration header.
    pub(crate) end_line: usize,
    /// The 1-based line of the `}` that closes the body, for types and methods with a body.
    pub(crate) body_end_line: Option<usize>,
    /// The index of the enclosing class or enum in [`ScriptOutline::items`]; `None` at top level.
    pub(crate) parent: Option<usize>,
    /// The attributes written before the declaration.
    pub(crate) attributes: Vec<ScriptAttribute>,
    /// The code of a method body, lines joined by spaces; empty for other kinds.
    pub(crate) body_code: String,
}

/// Every declaration of one script plus the lines that hold only attributes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ScriptOutline {
    /// Declarations in source order.
    pub(crate) items: Vec<ScriptItem>,
    /// 1-based lines an attribute spans, save the line its declaration starts on.
    pub(crate) attribute_lines: BTreeSet<usize>,
}

/// One open brace scope.
#[derive(Debug, Clone, Copy)]
enum Scope {
    Top,
    Class(usize),
    Enum(usize),
    /// A code block; `method` is the method whose body it belongs to, `root` marks its outermost block.
    Block {
        method: Option<usize>,
        root: bool,
    },
}

/// A member statement being read.
#[derive(Debug, Default)]
struct Pending {
    text: String,
    start_line: usize,
    paren_depth: i32,
    initializer_braces: i32,
    attributes: Vec<ScriptAttribute>,
    attribute: Option<(String, usize, i32)>,
}

/// Reads the declarations of `lines` into a [`ScriptOutline`].
///
/// Returns the outline; never fails, since malformed code yields fewer items rather than an error.
pub(crate) fn outline_script(lines: &[ScriptLine]) -> ScriptOutline {
    let mut parser = OutlineParser::default();
    for (index, line) in lines.iter().enumerate() {
        if line.code.trim_start().starts_with('#') {
            continue;
        }
        for c in line.code.chars() {
            parser.feed(c, index + 1);
        }
        parser.feed('\n', index + 1);
    }
    parser.outline
}

#[derive(Default)]
struct OutlineParser {
    outline: ScriptOutline,
    scopes: Vec<Scope>,
    pending: Pending,
}

impl OutlineParser {
    fn scope(&self) -> Scope {
        self.scopes.last().copied().unwrap_or(Scope::Top)
    }

    fn feed(&mut self, c: char, line: usize) {
        match self.scope() {
            Scope::Top => self.feed_member(c, line, None),
            Scope::Class(index) => self.feed_member(c, line, Some(index)),
            Scope::Enum(index) => self.feed_enum(c, line, index),
            Scope::Block { method, .. } => self.feed_block(c, line, method),
        }
    }

    fn feed_block(&mut self, c: char, line: usize, method: Option<usize>) {
        match c {
            '{' => self.scopes.push(Scope::Block {
                method,
                root: false,
            }),
            '}' => {
                if let Some(Scope::Block {
                    root: true,
                    method: Some(index),
                }) = self.scopes.pop()
                {
                    self.outline.items[index].body_end_line = Some(line);
                }
                return;
            }
            _ => {}
        }
        if let Some(index) = method {
            let body = &mut self.outline.items[index].body_code;
            body.push(if c == '\n' { ' ' } else { c });
        }
    }

    fn feed_enum(&mut self, c: char, line: usize, parent: usize) {
        match c {
            ',' | '}' => {
                self.finish_enum_member(line, parent);
                if c == '}' {
                    self.close_scope(line);
                }
            }
            _ if c.is_whitespace() => self.push_space(),
            _ => self.push_char(c, line),
        }
    }

    fn feed_member(&mut self, c: char, line: usize, parent: Option<usize>) {
        if let Some((text, start, depth)) = self.pending.attribute.as_mut() {
            text.push(if c == '\n' { ' ' } else { c });
            *depth += match c {
                '[' => 1,
                ']' => -1,
                _ => 0,
            };
            if *depth == 0 {
                let text = std::mem::take(text);
                let start = *start;
                self.pending.attribute = None;
                self.pending
                    .attributes
                    .push(attribute_from(text, start, line));
            }
            return;
        }
        let empty = self.pending.text.is_empty();
        match c {
            _ if c.is_whitespace() => self.push_space(),
            '[' if empty => self.pending.attribute = Some(("[".to_string(), line, 1)),
            '}' if self.pending.initializer_braces > 0 => {
                self.pending.initializer_braces -= 1;
                self.push_char(c, line);
            }
            '}' => {
                self.pending = Pending::default();
                self.close_scope(line);
            }
            '(' | ')' => {
                self.pending.paren_depth += if c == '(' { 1 } else { -1 };
                self.push_char(c, line);
            }
            ';' if self.pending.paren_depth == 0 && self.pending.initializer_braces == 0 => {
                self.finish_declaration(line, parent);
            }
            '{' if self.pending.paren_depth == 0
                && before_assignment(&self.pending.text).is_none() =>
            {
                self.finish_header(line, parent);
            }
            '{' => {
                self.pending.initializer_braces += 1;
                self.push_char(c, line);
            }
            _ => self.push_char(c, line),
        }
    }

    fn push_space(&mut self) {
        if !self.pending.text.is_empty() && !self.pending.text.ends_with(' ') {
            self.pending.text.push(' ');
        }
    }

    fn push_char(&mut self, c: char, line: usize) {
        if self.pending.text.is_empty() {
            self.pending.start_line = line;
        }
        self.pending.text.push(c);
    }

    fn close_scope(&mut self, line: usize) {
        match self.scopes.pop() {
            Some(Scope::Class(index)) | Some(Scope::Enum(index)) => {
                self.outline.items[index].body_end_line = Some(line);
            }
            Some(Scope::Block {
                method: Some(index),
                root: true,
            }) => {
                self.outline.items[index].body_end_line = Some(line);
            }
            _ => {}
        }
    }

    fn push_item(
        &mut self,
        kind: ItemKind,
        name: String,
        end_line: usize,
        parent: Option<usize>,
    ) -> usize {
        let pending = std::mem::take(&mut self.pending);
        for attribute in &pending.attributes {
            for spanned in
                attribute.start_line..=attribute.end_line.min(pending.start_line.saturating_sub(1))
            {
                self.outline.attribute_lines.insert(spanned);
            }
        }
        self.outline.items.push(ScriptItem {
            kind,
            name,
            start_line: pending.start_line,
            end_line,
            body_end_line: None,
            parent,
            attributes: pending.attributes,
            body_code: String::new(),
        });
        self.outline.items.len() - 1
    }

    fn finish_enum_member(&mut self, line: usize, parent: usize) {
        let text = self.pending.text.trim().to_string();
        if text.is_empty() {
            self.pending = Pending::default();
            return;
        }
        let name = first_identifier(&text);
        self.push_item(ItemKind::EnumMember, name, line, Some(parent));
    }

    fn finish_declaration(&mut self, line: usize, parent: Option<usize>) {
        let text = self.pending.text.trim().to_string();
        let first_word = text.split_whitespace().next().unwrap_or("");
        if text.is_empty() || matches!(first_word, "class" | "typedef" | "modded") {
            self.pending = Pending::default();
            return;
        }
        let head = before_assignment(&text).unwrap_or(&text);
        let (kind, name) = if head.contains('(') {
            (ItemKind::Method, name_before_paren(head))
        } else {
            (ItemKind::Field, last_identifier(head))
        };
        self.push_item(kind, name, line, parent);
    }

    fn finish_header(&mut self, line: usize, parent: Option<usize>) {
        let text = self.pending.text.trim().to_string();
        let words: Vec<&str> = text.split_whitespace().collect();
        let modded = words.first() == Some(&"modded");
        let rest: Vec<&str> = words
            .iter()
            .copied()
            .skip_while(|w| matches!(*w, "modded" | "sealed"))
            .collect();
        if rest.first() == Some(&"class") {
            let name = rest.get(1).map(|w| first_identifier(w)).unwrap_or_default();
            let base = class_base(&text);
            let index = self.push_item(ItemKind::Class { modded, base }, name, line, parent);
            self.scopes.push(Scope::Class(index));
        } else if rest.first() == Some(&"enum") {
            let name = rest.get(1).map(|w| first_identifier(w)).unwrap_or_default();
            let index = self.push_item(ItemKind::Enum, name, line, parent);
            self.scopes.push(Scope::Enum(index));
        } else if text.contains('(') {
            let name = name_before_paren(&text);
            let index = self.push_item(ItemKind::Method, name, line, parent);
            self.scopes.push(Scope::Block {
                method: Some(index),
                root: true,
            });
        } else {
            self.pending = Pending::default();
            self.scopes.push(Scope::Block {
                method: None,
                root: true,
            });
        }
    }
}

/// The text before the first `=` that sits outside parentheses, when there is such an `=`.
fn before_assignment(text: &str) -> Option<&str> {
    let mut depth = 0;
    for (index, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '=' if depth == 0 => return Some(&text[..index]),
            _ => {}
        }
    }
    None
}

fn attribute_from(text: String, start_line: usize, end_line: usize) -> ScriptAttribute {
    let name = first_identifier(text.trim_start_matches('[').trim_start());
    ScriptAttribute {
        name,
        text,
        start_line,
        end_line,
    }
}

fn first_identifier(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn last_identifier(text: &str) -> String {
    let trimmed = text.trim_end();
    let trimmed = match trimmed.strip_suffix(']') {
        Some(rest) => rest
            .rsplit_once('[')
            .map_or(rest, |(head, _)| head)
            .trim_end(),
        None => trimmed,
    };
    let reversed: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    reversed.chars().rev().collect()
}

fn name_before_paren(text: &str) -> String {
    let head = text.split('(').next().unwrap_or("");
    last_identifier(head)
}

fn class_base(text: &str) -> Option<String> {
    let after = if let Some((_, rest)) = text.split_once(':') {
        rest
    } else {
        text.split_once(" extends ")?.1
    };
    let base = first_identifier(after.trim());
    (!base.is_empty()).then_some(base)
}
