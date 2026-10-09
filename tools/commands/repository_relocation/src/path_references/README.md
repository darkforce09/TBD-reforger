# Path reference rewriting

The `path` row pass of the relocation: for one text file, every reference to a moved file or
folder, written from the repository root or relative to an anchor, rewritten to name the same file
after the moves.

## Contents

```text
tools/commands/repository_relocation/src/path_references/
├── anchor_resolution.rs    reads a relative literal from its anchors and re-relativises it
├── contents_block.rs       the tree part of a README's Contents block lines, live in a frozen index
├── markdown_links.rs       the link and image destinations of a Markdown document
├── mod.rs                  the pass: both edit kinds, unresolved literals, allowed spans
├── path_tokens.rs          segment boundaries, escapes, token ends, what one occurrence is
├── relative_references.rs  the relative literals of a file by its kind, each with its anchors
└── root_spellings.rs       `from/…`, `/from/…`, `\nfrom/…`, `prefix/from/…` rewritten by mapping
```

## How it works

`relative_references.rs` finds the candidates of a file by its kind: in Rust source the arguments of
`include!`, `include_str!`, `include_bytes!` and `#[path]`, literals built on
`CARGO_MANIFEST_DIR`, and every path token in other literals and comments; in Markdown every link
destination and every climbing token; in a Cargo manifest every `path = "…"` and every climbing
token; in any other text file every climbing token. A climbing token is led by `./` or `../`, led
by `/../` (a piece joined to a base, such as a `format!` or `concat!` argument; read without its
`/` from the file's or the crate's folder), or climbs back out of a folder it named
(`folder/../from`; read from the repository root too, since the root spelling pass leaves it
alone). A token that climbs back out of a folder it named counts under an anchor only where that
named lead (the segments before its first `..`) is a tracked folder: `7/../..`, a path-traversal test datum, names nothing from any
anchor without a folder `7`, however far its `..` segments reach. A token of `.` and `..` segments
alone (`../`, `./`, `../..`) is a candidate only where its syntax fixes the anchor (a link
destination, an `include!` or `#[path]` argument, a Cargo `path` value): in prose, a comment or a
plain string literal it speaks of a parent folder in general and is never rewritten.
`anchor_resolution.rs` reads each candidate from its anchors in order — the file's folder, the
owning crate's manifest folder, the repository root, and any crate folder only when none of those
names a tracked path. A reading that names a whole tracked path outranks one that names only a
leading part (a gitignored or planned tail), and a leading-part reading counts only for a literal
whose lead (`./`, `../`, `/../`) or place (a link destination, an `include!` or `#[path]` argument,
a Cargo `path` value, a `CARGO_MANIFEST_DIR` join) makes it relative: a plain token in a Rust
literal, such as a test's synthetic service unit path, is read from a folder only when its
whole path is tracked there, and otherwise left to the repository-root spelling pass. A literal of
separators alone (`/`) names no path; only `.` and `..` segments name a folder without a name.
When two anchors name different paths and would
rewrite the literal differently, or a literal that resolved before has no reading after the moves,
or a `./`, `../` or `/../` literal names a moved path but resolves under none of its anchors, the
literal is reported unresolved. A reading from the owning crate's folder is re-read from the crate
the planned tree gives the moved file (the nearest folder holding a crate manifest after the moves,
a manifest a row moves there or an untracked one on disk counting), and is unresolved when no folder
below the repository root holds one: the root manifest is the workspace's, so such a literal is
never re-anchored at the root. Otherwise the new literal names the moved target from the same
anchor, so a depth change re-relativises it in both directions; a literal that climbs back out of
a folder it named keeps everything through its last `..` (`folder/../to`).

A literal whose syntax does not fix its anchor (every candidate read under more than one anchor: a
path token in a Rust literal or comment, a climbing token in prose or a Cargo manifest) is
rewritten only when its spelling pins it to the reading that changes it. It is left as written
and reported ambiguous, never unresolved, when that reading names only a leading part of it (an
example path whose tail names nothing, such as a comment's ../../tests/cases_1.rs), or when the
reading comes from the owning crate's folder and another crate folder reads the same literal as a
tracked path the moves leave differently: a literal every crate spells for its own files
(`src/lib.rs`, `src/`) or a fixture path relative to a temporary checkout
(`../../engine/map` in a test's synthetic `Cargo.toml`) names no crate in particular. An
`include!`, `#[path]`, Markdown link, Cargo `path` value or `CARGO_MANIFEST_DIR` join fixes its
anchor and follows the moves as before.

`root_spellings.rs` rewrites every whole-path occurrence written from the repository root, after a
leading `/` (repository-root Markdown links), after the letter of a control escape (`\n`, `\t`,
`\r`, `\0` after an odd run of backslashes, as in `"a.rs\0from/b.rs"` test listings and messages
split over lines) or behind a deployment prefix (`/srv/checkout/from`, and everything after
`file://` in a `file:` URL, as in systemd's `Documentation=file:///…/from/…` or the host-less
`Documentation=file://from/…`), through the mapping. An escape-adjacent
occurrence whose escape letter and spelling together name a tracked path (`\nfrom` beside a
tracked `nfrom/`) has two readings and is reported unresolved. `path_tokens.rs` decides, the same
way for this pass and for the verification, whether an occurrence is a path at all: segment
boundaries on both sides, escape letters as boundaries, URLs other than `file:` URLs left alone.
`mod.rs` merges both edit lists, a relative edit outranking a root edit over the same bytes, and
keeps only the edits and unresolved items inside the spans the file's treatment opens (link
destinations in a frozen record; those and the tree part of the Contents block's lines, found by
`contents_block.rs`, in a frozen area's README index; the `spec`, `plan` and `owns` values of a
closed ticket; nothing in this crate's own test sources, whose literals and comments are fixture
text).

## Boundaries

- Depends on: `super::path_mapping`, `super::repository_files`, `super::rust_lexer`,
  `super::text_edits` and `super::file_treatment`.
- Used by: `super::relocation_plan` (the pass) and `super::retired_spellings` (`allowed_spans`,
  `path_tokens::classify_occurrence`).
- Rules: Rust code outside literals and comments is never edited; a bare `../` in prose or a
  comment is no candidate; a URL, a fragment-only link and an
  absolute path are never relative candidates; a literal of separators alone and a plain Rust
  token whose whole path is untracked name nothing, and so does a climbing token whose lead
  segment is no tracked folder at the anchor; a literal whose
  meaning the moves do not change is never rewritten; a literal its spelling does
  not pin to one anchor is left as written and reported ambiguous; escape-adjacent spellings,
  `file:` URLs and `/../` pieces are rewritten or unresolved, never skipped.
