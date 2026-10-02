**Status:** live

# Relocation manifests

The manifests the restructure program's stages run through `cargo xtask refactor relocate`: each one
lists the moves and reference rewrites of one stage, so no path is ever rewritten by hand, and stays
committed afterwards so the verification keeps proving that no live file spells what it retired.

## Contents

```text
documentation_v2/restructure/manifests/
└── example.tsv  the commented format sample the tests run; never judged as a stage manifest
```

## How it works

### Format

A manifest is a UTF-8, tab-separated `.tsv` file. Blank lines and lines starting with `#` are
skipped; the first other line is the header `kind`, `from`, `to`, `scope` (tab-separated).
Every row has three or four columns, and a manifest with any invalid row is refused as a whole,
every error named with its line. Paths are repository-relative, `/`-separated, and name the tree
as it stands before the manifest's moves.

| Kind | `from` and `to` | `scope` | What it does |
|---|---|---|---|
| `path` | a tracked file or folder, and where it goes | empty | `git mv` (parent folders created), then rewrites every reference to it in every tracked text file |
| `rust_path` | a Rust path prefix ending in `::`, such as `crate::v2::core::` | a folder whose `.rs` files are rewritten; empty means every tracked `.rs` | rewrites the prefix in `use` trees, code, attributes, doc links, comments and string literals |
| `text` | an identifier-like token, such as a package name | a folder, a glob holding `*` or `?`, or empty for every live text file | rewrites the token where neither neighbour is a letter, a digit, `_` or `-` |

A `path` row rewrites repository-root spellings (`from/…`, the `/from/…` of repository-root Markdown
links, and `from` behind a deployment prefix in strings, TOML, YAML, systemd units, `.gitattributes`
and `.gitignore`) and relative references (`include_str!` and its kin, `#[path]`, literals built on
`CARGO_MANIFEST_DIR`, Cargo `path = "…"`, Markdown link destinations and any `./` or `../` token).
A relative reference is read from the file's folder, the owning crate's folder or the repository
root, and rewritten so it names the moved target from the same anchor; one that cannot be re-read,
or that two anchors read differently, makes `--apply` refuse the whole manifest with its
`path:line` before anything is written. When a moved module's code reaches outside the moved
subtree through `self::` or `super::` chains, a `rust_path` row turns those chains into absolute
`crate::` paths first. Rows of one manifest compose: a path moves by the longest `from` that
contains it.

Frozen records change as little as their checks need: Markdown under the archive and the ticket
documents gets only its link destinations rewritten, prose and backticks staying as history; a
ticket record whose status is shipped or cancelled gets only its `spec` and `plan` lines rewritten.
Binary files and Git LFS pointers move with their folders and are never edited.

### Running a stage manifest

1. `cargo xtask refactor relocate --manifest <path.tsv> --dry-run` prints, per row, the tracked
   files moved and the references rewritten by file kind, then every unresolved literal; it
   writes nothing.
2. `cargo xtask refactor relocate --manifest <path.tsv> --apply` makes the moves and rewrites and
   then verifies the manifest.
3. `cargo xtask refactor relocate --verify` judges every committed manifest in this folder except
   `example.tsv`: no live tracked file spells a `path` row's retired `from` on segment boundaries
   (frozen records and the manifests themselves excluded), and no `rust_path` prefix is left in its
   scope. Exit 0 pass, 1 findings, 2 did not run.

A stage commits its manifest here with the moves it made, named after the stage (for example
`s01_top_level_folders.tsv`), and never edits it afterwards.

## Code

- [Relocation](/tools_v2/xtask/src/commands/refactor/relocate/) — the parser, the passes, the moves
  and the verification that read these files.

## Boundaries

- Depends on: the relocation tool's manifest parser, which defines the format above.
- Used by: `cargo xtask refactor relocate --verify`, which judges every stage manifest here; the
  relocation tests, which run `example.tsv` on a throwaway checkout.
- Rules: `example.tsv` stays the format sample with one row of each kind
  (`relocate_example_manifest_parses_and_applies`); a committed stage manifest is never edited,
  since its rows are the retired spellings the verification keeps out of the tree.

## Related documentation

- [Laws and gates](/documentation_v2/restructure/laws_and_gates.md) — the relocation law the
  verification enforces between stages.
- [Path coupling research](/documentation_v2/archive/restructure_research/02_path_coupling.md) —
  every kind of path reference a move breaks.
