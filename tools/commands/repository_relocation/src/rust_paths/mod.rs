//! The `rust_path` row pass: every Rust path that starts with a retired prefix, in one file.
//!
//! **Role:** rewrites one Rust source file for the manifest's [`path_rules::RustPathRules`]:
//! `use` trees leaf by leaf ([`use_trees`]), regrouping a tree whose retired prefix spans a `{`;
//! paths in code, attributes and expressions; and the same prefixes inside comments (doc links
//! such as `[`crate::a::B`]`) and string literals. A `self::` or `super::` path that crosses the
//! edge of a moved `crate::` module becomes an absolute `crate::` path first, read against the
//! file's module from the [`module_tree::ModuleTree`] and the inline modules around it.
//!
//! **Position:** called by the plan builder ([`super::relocation_plan`]) for every live `.rs` file
//! inside a `rust_path` row's scope, after the path-row pass; the verification
//! ([`super::retired_spellings`]) runs it without a module to find surviving prefixes.
//!
//! **Signals & state:** none; pure functions over one file's text.
//!
//! **Invariants:** a leaf that imported a module under its own name keeps that name (`as old`)
//! when the move renames the module; a tree that must be regrouped but holds a comment or a leading
//! `::` is reported, never rewritten with the comment lost; edits never overlap.

pub(crate) mod module_tree;
pub(crate) mod path_rules;
pub(crate) mod use_trees;

use std::ops::Range;

use module_tree::{module_path_at, scan_modules};
use path_rules::RustPathRules;
use use_trees::{RenderedLeaf, UseDeclaration, UseLeaf, render_use_tree, use_declarations};

use super::path_references::path_tokens::match_starts;
use super::rust_lexer::{Token, TokenKind, code_tokens, string_content, tokenize};
use super::text_edits::{Edit, merge_edits};

/// One file's Rust path edits and the `use` trees that could not be rewritten.
#[derive(Debug, Default)]
pub(crate) struct RustPathOutcome {
    /// The edits, merged and in source order.
    pub(crate) edits: Vec<Edit>,
    /// The byte offset and reason of every `use` tree left unrewritten.
    pub(crate) unresolved: Vec<(usize, String)>,
}

/// The edits that rewrite every retired prefix of `rules` in `source`; `file_module` is the
/// file's module path below `crate` when the crate's module tree reaches it.
pub(crate) fn rust_path_edits(
    source: &str,
    rules: &RustPathRules,
    file_module: Option<&[String]>,
) -> RustPathOutcome {
    let mut outcome = RustPathOutcome::default();
    if rules.is_empty() {
        return outcome;
    }
    let tokens = tokenize(source);
    let (_, inline) = scan_modules(source);
    let module_at =
        |offset: usize| file_module.map(|module| module_path_at(module, &inline, offset));
    let declarations = use_declarations(source, &tokens);
    let mut edits = Vec::new();
    let mut regrouped: Vec<Range<usize>> = Vec::new();
    for declaration in &declarations {
        let module = module_at(declaration.keyword_start);
        match use_declaration_edits(declaration, rules, module.as_deref()) {
            UseRewrite::Unchanged => {}
            UseRewrite::InPlace(found) => edits.extend(found),
            UseRewrite::Regrouped(edit) => {
                regrouped.push(edit.span.clone());
                edits.push(edit);
            }
            UseRewrite::Unrewritable(message) => {
                outcome
                    .unresolved
                    .push((declaration.keyword_start, message));
            }
        }
    }
    let use_spans: Vec<Range<usize>> = declarations.iter().map(|d| d.tree.clone()).collect();
    edits.extend(code_path_edits(
        source, &tokens, &use_spans, rules, &module_at,
    ));
    for token in &tokens {
        let span = match token.kind {
            TokenKind::StringLiteral => string_content(source, token),
            TokenKind::LineComment | TokenKind::BlockComment => token.start..token.end,
            _ => continue,
        };
        if !regrouped
            .iter()
            .any(|r| r.start <= span.start && span.end <= r.end)
        {
            edits.extend(textual_prefix_edits(source, span, rules));
        }
    }
    outcome.edits = merge_edits(source, edits);
    outcome
}

/// What one `use` declaration needs.
enum UseRewrite {
    Unchanged,
    InPlace(Vec<Edit>),
    Regrouped(Edit),
    Unrewritable(String),
}

fn use_declaration_edits(
    declaration: &UseDeclaration,
    rules: &RustPathRules,
    module: Option<&[String]>,
) -> UseRewrite {
    let rewritten: Vec<Option<(usize, Vec<String>)>> = declaration
        .leaves
        .iter()
        .map(|leaf| rules.rewrite(&texts(leaf), module))
        .collect();
    let Some(row_line) = rewritten.iter().flatten().map(|(row, _)| *row).next() else {
        return UseRewrite::Unchanged;
    };
    if let Some(edits) = in_place_edits(&declaration.leaves, &rewritten) {
        return UseRewrite::InPlace(edits);
    }
    if declaration.has_comments || declaration.leading_colons {
        return UseRewrite::Unrewritable(
            "a `use` tree whose retired prefix spans a `{` holds a comment or a leading `::`; \
             split it so the prefix sits in one straight chain"
                .to_string(),
        );
    }
    let leaves: Vec<RenderedLeaf> = declaration
        .leaves
        .iter()
        .zip(&rewritten)
        .map(|(leaf, change)| {
            let old = texts(leaf);
            match change {
                Some((_, new)) => RenderedLeaf {
                    rename: leaf.rename.clone().or_else(|| kept_binding(&old, new)),
                    segments: new.clone(),
                },
                None => RenderedLeaf {
                    segments: old,
                    rename: leaf.rename.clone(),
                },
            }
        })
        .collect();
    UseRewrite::Regrouped(Edit {
        span: declaration.tree.clone(),
        replacement: render_use_tree(&leaves),
        row_line,
    })
}

/// The edits that rewrite every changed leaf inside one straight chain, or `None` when a change
/// spans a `{`, two leaves need different text over the same bytes, or an edit would also change a
/// leaf that shares its chain but must stay.
fn in_place_edits(
    leaves: &[UseLeaf],
    rewritten: &[Option<(usize, Vec<String>)>],
) -> Option<Vec<Edit>> {
    let mut edits: Vec<Edit> = Vec::new();
    for (leaf, change) in leaves.iter().zip(rewritten) {
        let Some((row_line, new)) = change else {
            continue;
        };
        let old = texts(leaf);
        let shared = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take_while(|(a, b)| a == b)
            .count()
            .min(old.len().saturating_sub(1));
        let changed = &leaf.segments[..old.len() - shared];
        if changed
            .iter()
            .any(|segment| segment.chain != changed[0].chain)
        {
            return None;
        }
        let span = changed[0].span.start..changed[changed.len() - 1].span.end;
        let replacement = new[..new.len() - shared].join("::");
        if edits
            .iter()
            .any(|e| e.span == span && e.replacement != replacement)
        {
            return None;
        }
        edits.push(Edit {
            span,
            replacement,
            row_line: *row_line,
        });
        if let Some(binding) = leaf
            .rename
            .is_none()
            .then(|| kept_binding(&old, new))
            .flatten()
        {
            let end = leaf.segments[leaf.segments.len() - 1].span.end;
            edits.push(Edit {
                span: end..end,
                replacement: format!(" as {binding}"),
                row_line: *row_line,
            });
        }
    }
    edits.dedup();
    let every_leaf_lands = leaves.iter().zip(rewritten).all(|(leaf, change)| {
        let expected = change
            .as_ref()
            .map_or_else(|| texts(leaf), |(_, new)| new.clone());
        leaf_after_edits(leaf, &edits) == expected
    });
    every_leaf_lands.then_some(edits)
}

/// The segments `leaf` holds once `edits` are applied to the tree's text.
fn leaf_after_edits(leaf: &UseLeaf, edits: &[Edit]) -> Vec<String> {
    let mut result = Vec::new();
    let mut skip_until = 0;
    for segment in &leaf.segments {
        if segment.span.start < skip_until {
            continue;
        }
        let replaced = edits
            .iter()
            .find(|edit| !edit.span.is_empty() && edit.span.start == segment.span.start);
        match replaced {
            Some(edit) => {
                result.extend(edit.replacement.split("::").map(str::to_string));
                skip_until = edit.span.end;
            }
            None => result.push(segment.text.clone()),
        }
    }
    result
}

/// The name a leaf binds: its last segment, or the one before a trailing `self`.
fn binding(segments: &[String]) -> Option<&str> {
    match segments.split_last()? {
        (last, _) if last == "*" => None,
        (last, parents) if last == "self" => parents.last().map(String::as_str),
        (last, _) => Some(last),
    }
}

/// The old binding, when the rewrite would change the name the leaf binds.
fn kept_binding(old: &[String], new: &[String]) -> Option<String> {
    let before = binding(old)?;
    (binding(new) != Some(before)).then(|| before.to_string())
}

fn texts(leaf: &UseLeaf) -> Vec<String> {
    leaf.segments.iter().map(|s| s.text.clone()).collect()
}

/// The edits for paths in code outside `use` trees.
fn code_path_edits(
    source: &str,
    tokens: &[Token],
    use_spans: &[Range<usize>],
    rules: &RustPathRules,
    module_at: &dyn Fn(usize) -> Option<Vec<String>>,
) -> Vec<Edit> {
    let code = code_tokens(tokens);
    let separator = |at: usize| {
        at + 1 < code.len()
            && code[at].is_punctuation(source, ':')
            && code[at + 1].is_punctuation(source, ':')
            && code[at + 1].start == code[at].end
    };
    let mut edits = Vec::new();
    let mut index = 0;
    while index < code.len() {
        let token = code[index];
        let in_use = use_spans
            .iter()
            .any(|s| s.start <= token.start && token.end <= s.end);
        let starts_path =
            token.kind == TokenKind::Identifier && !(index >= 2 && separator(index - 2));
        if in_use || !starts_path {
            index += 1;
            continue;
        }
        let mut last = index;
        while separator(last + 1)
            && code
                .get(last + 3)
                .is_some_and(|t| t.kind == TokenKind::Identifier)
        {
            last += 3;
        }
        let segments: Vec<String> = (index..=last)
            .step_by(3)
            .map(|at| code[at].text(source).to_string())
            .collect();
        if segments.len() >= 2 {
            let module = module_at(token.start);
            if let Some((row_line, new)) = rules.rewrite(&segments, module.as_deref()) {
                edits.push(Edit {
                    span: token.start..code[last].end,
                    replacement: new.join("::"),
                    row_line,
                });
            }
        }
        index = last + 1;
    }
    edits
}

/// The edits for absolute retired prefixes written inside one comment or string literal.
fn textual_prefix_edits(source: &str, span: Range<usize>, rules: &RustPathRules) -> Vec<Edit> {
    let text = &source[span.clone()];
    let is_identifier = |c: char| c.is_alphanumeric() || c == '_';
    let mut edits = Vec::new();
    for rule in rules.rules() {
        let from = rule.retired_prefix_text();
        for at in match_starts(text, &from) {
            let before = text[..at].chars().next_back();
            let after = &text[at + from.len()..];
            let starts_clean = before.is_none_or(|c| !is_identifier(c) && c != ':');
            let ends_clean = match after.chars().next() {
                Some(':') => after.starts_with("::"),
                Some(c) => !is_identifier(c),
                None => true,
            };
            if starts_clean && ends_clean {
                let start = span.start + at;
                edits.push(Edit {
                    span: start..start + from.len(),
                    replacement: rule.new_prefix_text(),
                    row_line: rule.row_line,
                });
            }
        }
    }
    edits.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then(b.span.end.cmp(&a.span.end))
    });
    edits
}
