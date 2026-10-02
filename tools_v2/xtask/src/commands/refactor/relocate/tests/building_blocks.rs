//! The pure building blocks of the relocation: the lexer, the manifest parser, globs, path math,
//! occurrence classes, Markdown links, `use` trees and Rust path rules.

use super::manifest::{RowKind, RowScope, glob_matches, parse_manifest};
use super::path_mapping::{PathMapping, normalize, relative_path};
use super::path_references::markdown_links::link_destinations;
use super::path_references::path_tokens::{PathOccurrence, classify_occurrence};
use super::rust_lexer::{TokenKind, string_content, tokenize};
use super::rust_paths::path_rules::{RustPathRules, segments_of};
use super::rust_paths::use_trees::{RenderedLeaf, render_use_tree, use_declarations};

#[test]
fn relocate_lexer_tells_literals_lifetimes_and_comments_apart() {
    let source = "fn f<'a>(x: &'a str) -> char { /* a /* nested */ \" */ let r = r#\"say \"hi\"\"#; 'q' } // tail \"x\"";
    let tokens = tokenize(source);
    let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
    assert!(kinds.contains(&TokenKind::Lifetime));
    assert!(kinds.contains(&TokenKind::CharacterLiteral));
    let comments: Vec<&str> = tokens
        .iter()
        .filter(|t| t.is_comment())
        .map(|t| t.text(source))
        .collect();
    assert_eq!(comments, vec!["/* a /* nested */ \" */", "// tail \"x\""]);
    let raw = tokens
        .iter()
        .find(|t| t.kind == TokenKind::StringLiteral)
        .expect("the raw string");
    assert_eq!(&source[string_content(source, raw)], "say \"hi\"");
}

#[test]
fn relocate_manifest_parses_rows_and_refuses_bad_ones_by_line() {
    let good = "# comment\n\nkind\tfrom\tto\tscope\npath\ta/b/\tc\t\nrust_path\tcrate::x::\tcrate::y::\tapps\ntext\tname-a\tname\tapps/**/*.toml\n";
    let rows = parse_manifest(good).expect("valid manifest");
    assert_eq!(rows.len(), 3);
    assert_eq!(
        (rows[0].kind, rows[0].from.as_str(), rows[0].line),
        (RowKind::Path, "a/b", 4)
    );
    assert_eq!(rows[1].scope, RowScope::Folder("apps".to_string()));
    assert_eq!(rows[2].scope, RowScope::Glob("apps/**/*.toml".to_string()));

    let bad = "kind\tfrom\tto\tscope\npath\t../a\tb\t\nrust_path\tcrate::x\tcrate::y::\t\nmove\ta\tb\t\npath\ta\ta/b\t\npath\tx\ty\tscope\n";
    let errors = parse_manifest(bad).expect_err("five bad rows");
    assert_eq!(errors.len(), 5, "{errors:?}");
    assert!(errors[0].starts_with("line 2:"));
    assert!(errors[3].contains("cannot move into itself"));
    assert!(
        parse_manifest("path\ta\tb\t\n").is_err(),
        "the header row is required"
    );
}

#[test]
fn relocate_globs_keep_single_stars_inside_one_name() {
    assert!(glob_matches(b"apps/**/*.toml", b"apps/a/b/c.toml"));
    assert!(glob_matches(b"apps/**/*.toml", b"apps/c.toml"));
    assert!(!glob_matches(b"apps/*.toml", b"apps/a/c.toml"));
    assert!(glob_matches(b"src/bin/*/main.rs", b"src/bin/tool/main.rs"));
    assert!(!glob_matches(b"src/bin/*.rs", b"src/bin/tool/main.rs"));
}

#[test]
fn relocate_path_math_normalises_and_relativises() {
    assert_eq!(
        normalize("apps/web/api/src", "../../../../data/x"),
        Some("data/x".to_string())
    );
    assert_eq!(normalize("apps", "../../x"), None);
    assert_eq!(relative_path("apps/api/src", "data/x"), "../../../data/x");
    assert_eq!(relative_path("a/b", "a/b"), ".");
    assert_eq!(relative_path("", "a/b"), "a/b");
}

#[test]
fn relocate_mapping_relocates_by_the_longest_row() {
    let rows = parse_manifest("kind\tfrom\tto\tscope\npath\tdocs_root\tdocumentation\t\npath\tdocs_root/assets_old\tdocumentation/assets\t\n")
        .expect("valid manifest");
    let mapping = PathMapping::from_rows(&rows);
    assert_eq!(
        mapping.relocate("docs_root/assets_old/x.md"),
        "documentation/assets/x.md"
    );
    assert_eq!(
        mapping.relocate("docs_root/runbook.md"),
        "documentation/runbook.md"
    );
    assert_eq!(mapping.relocate("docs_rootx/a"), "docs_rootx/a");
}

#[test]
fn relocate_occurrences_are_classified_by_their_neighbours() {
    let class = |text: &str, needle: &str| {
        let start = text.find(needle).expect("needle");
        classify_occurrence(text, start, start + needle.len())
    };
    assert_eq!(class("see old/x", "old"), PathOccurrence::RepositoryRoot);
    assert_eq!(class("[a](/old/x)", "old"), PathOccurrence::RepositoryRoot);
    assert_eq!(
        class("x ../../old/y", "old"),
        PathOccurrence::Relative { token_start: 2 }
    );
    assert_eq!(
        class("x guides/old/y", "old"),
        PathOccurrence::Embedded { token_start: 2 }
    );
    assert_eq!(class("https://h/old/y", "old"), PathOccurrence::Url);
    assert_eq!(class("my_old/y", "old"), PathOccurrence::NotAPath);
    assert_eq!(class("old.md", "old"), PathOccurrence::NotAPath);
    assert_eq!(class("end old.", "old"), PathOccurrence::RepositoryRoot);
}

#[test]
fn relocate_markdown_links_skip_code_spans_and_fences() {
    let text = "[a](x/a.md) `[b](x/b.md)` ![c](<x/c d.md> \"t\")\n```text\n[d](x/d.md)\n```\n[e]: x/e.md\n";
    let destinations: Vec<&str> = link_destinations(text)
        .into_iter()
        .map(|span| &text[span])
        .collect();
    assert_eq!(destinations, vec!["x/a.md", "x/c d.md", "x/e.md"]);
}

#[test]
fn relocate_use_trees_flatten_and_render() {
    let source = "pub(crate) use crate::a::{b::{self, C}, d as e, *};\nfn f() { use<'x> }\n";
    let declarations = use_declarations(source, &tokenize(source));
    assert_eq!(declarations.len(), 1);
    let leaves: Vec<Vec<String>> = declarations[0]
        .leaves
        .iter()
        .map(|leaf| leaf.segments.iter().map(|s| s.text.clone()).collect())
        .collect();
    assert_eq!(leaves.len(), 4);
    assert_eq!(leaves[1], vec!["crate", "a", "b", "C"]);
    assert_eq!(declarations[0].leaves[2].rename.as_deref(), Some("e"));
    let rendered = render_use_tree(&[
        RenderedLeaf {
            segments: segments_of("crate::x::self"),
            rename: Some("old".to_string()),
        },
        RenderedLeaf {
            segments: segments_of("crate::x::Y"),
            rename: None,
        },
        RenderedLeaf {
            segments: segments_of("crate::z::W"),
            rename: None,
        },
    ]);
    assert_eq!(rendered, "crate::{x::{self as old, Y}, z::W}");
}

#[test]
fn relocate_rust_rules_resolve_super_chains_only_across_a_moved_edge() {
    let rows = parse_manifest(
        "kind\tfrom\tto\tscope\nrust_path\tcrate::outer::moved::\tcrate::moved::\t\n",
    )
    .expect("valid manifest");
    let rules = RustPathRules::from_rows(&rows);
    let module: Vec<String> = segments_of("outer::moved::deep");
    let rewrite = |path: &str| {
        rules
            .rewrite(&segments_of(path), Some(&module))
            .map(|(_, s)| s.join("::"))
    };
    assert_eq!(
        rewrite("super::Sibling"),
        None,
        "stays inside the moved module"
    );
    assert_eq!(
        rewrite("super::super::other::X"),
        Some("crate::outer::other::X".to_string())
    );
    assert_eq!(
        rewrite("crate::outer::moved::A"),
        Some("crate::moved::A".to_string())
    );
    let outside: Vec<String> = segments_of("outer");
    assert_eq!(
        rules
            .rewrite(&segments_of("self::moved::A"), Some(&outside))
            .map(|(_, s)| s.join("::")),
        Some("crate::moved::A".to_string())
    );
}
