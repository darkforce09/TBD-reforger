# Path reference rewriting

The `path` row pass of the relocation: for one text file, every reference to a moved file or
folder, written from the repository root or relative to an anchor, rewritten to name the same file
after the moves.

## Contents

```text
tools_v2/xtask/src/commands/refactor/relocate/path_references/
├── anchor_resolution.rs    reads a relative literal from its anchors and re-relativises it
├── markdown_links.rs       the link and image destinations of a Markdown document
├── mod.rs                  the pass: both edit kinds, unresolved literals, allowed spans
├── path_tokens.rs          segment boundaries, token ends, what one occurrence of a spelling is
├── relative_references.rs  the relative literals of a file by its kind, each with its anchors
└── root_spellings.rs       `from/…`, `/from/…` and `prefix/from/…` rewritten through the mapping
```

## How it works

`relative_references.rs` finds the candidates of a file by its kind: in Rust source the arguments of
`include!`, `include_str!`, `include_bytes!` and `#[path]`, literals built on
`CARGO_MANIFEST_DIR`, and every path token in other literals and comments; in Markdown every link
destination and every `./` or `../` token; in a Cargo manifest every `path = "…"`; in any other text
file every `./` or `../` token. `anchor_resolution.rs` reads each candidate from its anchors in
order — the file's folder, the owning crate's manifest folder, the repository root, and any crate
folder only when none of those names a tracked path. A reading that names a whole tracked path
outranks one that names only a leading part (a gitignored or planned tail). When two anchors name
different paths and would rewrite the literal differently, or a literal that resolved before has no
reading after the moves, the literal is reported unresolved. Otherwise the new literal names the
moved target from the same anchor, so a depth change re-relativises it in both directions.

`root_spellings.rs` rewrites every whole-path occurrence written from the repository root, after a
leading `/` (repository-root Markdown links) or behind a deployment prefix, through the mapping.
`path_tokens.rs` decides, the same way for this pass and for the verification, whether an
occurrence is a path at all: segment boundaries on both sides, URLs left alone. `mod.rs` merges both
edit lists, a relative edit outranking a root edit over the same bytes, and keeps only the edits
inside the spans the file's treatment opens (link destinations in a frozen record, the `spec` and
`plan` lines of a closed ticket).

## Boundaries

- Depends on: `super::path_mapping`, `super::repository_files`, `super::rust_lexer`,
  `super::text_edits` and `super::file_treatment`.
- Used by: `super::relocation_plan` (the pass) and `super::retired_spellings` (`allowed_spans`,
  `path_tokens::classify_occurrence`).
- Rules: Rust code outside literals and comments is never edited; a URL, a fragment-only link and an
  absolute path are never relative candidates; a literal whose meaning the moves do not change is
  never rewritten (`relocate_depth_change_rerelativises_include_and_manifest_dir_literals`,
  `relocate_unresolvable_literal_fails_apply_with_nothing_written`).
