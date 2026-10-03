# Path reference rewriting

The `path` row pass of the relocation: for one text file, every reference to a moved file or
folder, written from the repository root or relative to an anchor, rewritten to name the same file
after the moves.

## Contents

```text
tools/xtask/src/commands/refactor/relocate/path_references/
├── anchor_resolution.rs    reads a relative literal from its anchors and re-relativises it
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
alone).
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
literal is reported unresolved. Otherwise the new literal names the moved target from the same
anchor, so a depth change re-relativises it in both directions; a literal that climbs back out of
a folder it named keeps everything through its last `..` (`folder/../to`).

A literal whose syntax does not fix its anchor (every candidate read under more than one anchor: a
path token in a Rust literal or comment, a climbing token in prose or a Cargo manifest) is
rewritten only when its spelling pins it to the reading that changes it. It is left as written
and reported ambiguous, never unresolved, when that reading names only a leading part of it (an
example path whose tail names nothing, such as a comment's `../../tests/cases_1.rs`), or when the
reading comes from the owning crate's folder and another crate folder reads the same literal as a
tracked path the moves leave differently: a literal every crate spells for its own files
(`src/lib.rs`, `src/`) or a fixture path relative to a temporary checkout
(`../../legacy/map_engine` in a test's synthetic `Cargo.toml`) names no crate in particular. An
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
destinations in a frozen record, the `spec`, `plan` and `owns` values of a closed ticket).

## Boundaries

- Depends on: `super::path_mapping`, `super::repository_files`, `super::rust_lexer`,
  `super::text_edits` and `super::file_treatment`.
- Used by: `super::relocation_plan` (the pass) and `super::retired_spellings` (`allowed_spans`,
  `path_tokens::classify_occurrence`).
- Rules: Rust code outside literals and comments is never edited; a URL, a fragment-only link and an
  absolute path are never relative candidates; a literal of separators alone and a plain Rust
  token whose whole path is untracked name nothing
  (`relocate_lone_separator_literals_name_no_path`,
  `relocate_plain_fixture_paths_under_a_moved_folder_name_stay_as_written`); a literal whose
  meaning the moves do not change is never rewritten (`relocate_depth_change_rerelativises_include_and_manifest_dir_literals`,
  `relocate_unresolvable_literal_fails_apply_with_nothing_written`); a literal its spelling does
  not pin to one anchor is left as written and reported ambiguous
  (`relocate_crate_generic_literals_in_a_file_leaving_its_crate_stay_as_written`); escape-adjacent spellings,
  `file:` URLs and `/../` pieces are rewritten or unresolved, never skipped
  (`relocate_spellings_after_control_escapes_are_rewritten_and_verified`,
  `relocate_file_urls_carrying_a_repository_path_are_rewritten`,
  `relocate_slash_led_climbs_in_macro_arguments_follow_their_anchor`).
