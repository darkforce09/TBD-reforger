//! `use` declarations as lists of leaves, and leaves back into one grouped `use` tree.
//!
//! **Role:** finds every `use` declaration of a file, flattens its tree into [`UseLeaf`]s (one per
//! imported name, glob or `self`, each segment with its byte span and the straight chain it sits
//! in) and renders a list of leaves as one grouped tree (`a::{b, c::{d, e}}`).
//!
//! **Position:** read by the Rust path pass ([`super`]), which rewrites leaves in place when the
//! rewritten segments form one straight chain and regroups the whole tree when they do not.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** two segments share a chain id exactly when no `{` stands between them, so a
//! textual replacement over one chain changes every leaf below it the same way; the rendered tree
//! imports exactly the leaves it is given, in their first-seen order; `use<…>` capture lists are no
//! declarations.

use std::ops::Range;

use super::super::rust_lexer::{Token, TokenKind};

/// One segment of a leaf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LeafSegment {
    /// The segment as written: a name, `self`, `super`, `crate` or `*`.
    pub(crate) text: String,
    /// Its bytes.
    pub(crate) span: Range<usize>,
    /// The straight chain it sits in.
    pub(crate) chain: usize,
}

/// One imported name of a `use` tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UseLeaf {
    /// Every segment from the tree's root to the leaf, the imported name last.
    pub(crate) segments: Vec<LeafSegment>,
    /// The `as` name, when the leaf has one.
    pub(crate) rename: Option<String>,
}

/// One `use` declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UseDeclaration {
    /// The `use` keyword's first byte.
    pub(crate) keyword_start: usize,
    /// The tree's bytes: from after `use` to before `;`.
    pub(crate) tree: Range<usize>,
    /// The leaves, in source order.
    pub(crate) leaves: Vec<UseLeaf>,
    /// Whether a comment sits inside the tree.
    pub(crate) has_comments: bool,
    /// Whether the tree starts with `::`.
    pub(crate) leading_colons: bool,
}

/// Every `use` declaration in `tokens` (all tokens of `source`, comments included).
pub(crate) fn use_declarations(source: &str, tokens: &[Token]) -> Vec<UseDeclaration> {
    let mut found = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        let next_code = tokens[index + 1..].iter().find(|t| !t.is_comment());
        let is_declaration = token.is_word(source, "use")
            && next_code.is_some_and(|next| !next.is_punctuation(source, '<'));
        if !is_declaration {
            index += 1;
            continue;
        }
        let Some(end) =
            (index + 1..tokens.len()).find(|at| tokens[*at].is_punctuation(source, ';'))
        else {
            break;
        };
        let inside = &tokens[index + 1..end];
        let code: Vec<Token> = inside.iter().copied().filter(|t| !t.is_comment()).collect();
        if let (Some(first), Some(last)) = (code.first(), code.last()) {
            let mut parser = TreeParser {
                source,
                code: &code,
                at: 0,
                next_chain: 0,
                leaves: Vec::new(),
                leading_colons: false,
            };
            if parser.parse_tree(Vec::new()) && parser.at == code.len() {
                found.push(UseDeclaration {
                    keyword_start: token.start,
                    tree: first.start..last.end,
                    leaves: parser.leaves,
                    has_comments: inside.iter().any(Token::is_comment),
                    leading_colons: parser.leading_colons,
                });
            }
        }
        index = end + 1;
    }
    found
}

struct TreeParser<'a> {
    source: &'a str,
    code: &'a [Token],
    at: usize,
    next_chain: usize,
    leaves: Vec<UseLeaf>,
    leading_colons: bool,
}

impl TreeParser<'_> {
    fn peek_is(&self, character: char) -> bool {
        self.code
            .get(self.at)
            .is_some_and(|t| t.is_punctuation(self.source, character))
    }

    fn path_separator(&self) -> bool {
        self.peek_is(':')
            && self.code.get(self.at + 1).is_some_and(|second| {
                second.is_punctuation(self.source, ':') && second.start == self.code[self.at].end
            })
    }

    fn new_chain(&mut self) -> usize {
        self.next_chain += 1;
        self.next_chain
    }

    /// Parse one tree below `prefix`; `false` on a token the grammar does not allow.
    fn parse_tree(&mut self, mut prefix: Vec<LeafSegment>) -> bool {
        let chain = self.new_chain();
        if prefix.is_empty() && self.path_separator() {
            self.leading_colons = true;
            self.at += 2;
        }
        loop {
            let Some(token) = self.code.get(self.at).copied() else {
                return false;
            };
            if token.is_punctuation(self.source, '{') {
                self.at += 1;
                while !self.peek_is('}') {
                    if !self.parse_tree(prefix.clone()) {
                        return false;
                    }
                    if self.peek_is(',') {
                        self.at += 1;
                    } else if !self.peek_is('}') {
                        return false;
                    }
                }
                self.at += 1;
                return true;
            }
            let is_glob = token.is_punctuation(self.source, '*');
            if !is_glob && token.kind != TokenKind::Identifier {
                return false;
            }
            self.at += 1;
            prefix.push(LeafSegment {
                text: token.text(self.source).to_string(),
                span: token.start..token.end,
                chain,
            });
            if !is_glob && self.path_separator() {
                self.at += 2;
                continue;
            }
            let mut rename = None;
            if !is_glob
                && self
                    .code
                    .get(self.at)
                    .is_some_and(|t| t.is_word(self.source, "as"))
            {
                let alias = self.code.get(self.at + 1).copied();
                let Some(alias) = alias.filter(|a| a.kind == TokenKind::Identifier) else {
                    return false;
                };
                rename = Some(alias.text(self.source).to_string());
                self.at += 2;
            }
            self.leaves.push(UseLeaf {
                segments: prefix,
                rename,
            });
            return true;
        }
    }
}

/// One leaf to render: its segments and its `as` name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RenderedLeaf {
    /// The segments, the imported name (or `self`, or `*`) last.
    pub(crate) segments: Vec<String>,
    /// The `as` name.
    pub(crate) rename: Option<String>,
}

/// A grouped tree that imports exactly `leaves`.
pub(crate) fn render_use_tree(leaves: &[RenderedLeaf]) -> String {
    let mut root = TrieNode::default();
    for leaf in leaves {
        let mut node = &mut root;
        let (last, parents) = leaf.segments.split_last().expect("a leaf has a segment");
        for segment in parents {
            node = node.child(segment);
        }
        if last == "self" {
            node.terminals.push(leaf.rename.clone());
        } else {
            node.child(last).terminals.push(leaf.rename.clone());
        }
    }
    let items: Vec<String> = root.children.iter().flat_map(TrieNode::items).collect();
    if items.len() == 1 {
        items.into_iter().next().unwrap_or_default()
    } else {
        format!("{{{}}}", items.join(", "))
    }
}

#[derive(Default)]
struct TrieNode {
    name: String,
    terminals: Vec<Option<String>>,
    children: Vec<TrieNode>,
}

impl TrieNode {
    fn child(&mut self, name: &str) -> &mut TrieNode {
        let position = match self.children.iter().position(|c| c.name == name) {
            Some(position) => position,
            None => {
                self.children.push(TrieNode {
                    name: name.to_string(),
                    ..TrieNode::default()
                });
                self.children.len() - 1
            }
        };
        &mut self.children[position]
    }

    /// The items this node contributes to its parent's group.
    fn items(&self) -> Vec<String> {
        let named = |rename: &Option<String>, name: &str| match rename {
            Some(alias) => format!("{name} as {alias}"),
            None => name.to_string(),
        };
        if self.children.is_empty() {
            return self
                .terminals
                .iter()
                .map(|r| named(r, &self.name))
                .collect();
        }
        let mut inner: Vec<String> = self.terminals.iter().map(|r| named(r, "self")).collect();
        inner.extend(self.children.iter().flat_map(TrieNode::items));
        if inner.len() == 1 {
            vec![format!("{}::{}", self.name, inner[0])]
        } else {
            vec![format!("{}::{{{}}}", self.name, inner.join(", "))]
        }
    }
}
