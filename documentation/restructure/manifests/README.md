**Status:** live

# Relocation manifests

The manifests the restructure program's stages run through `cargo xtask refactor relocate`: each one
lists the moves and reference rewrites of one stage, so no path is ever rewritten by hand, and stays
committed afterwards so the verification keeps proving that no live file spells what it retired.

## Contents

```text
documentation/restructure/manifests/
├── example.tsv               the commented format sample the tests run; never judged as a stage manifest
├── m2_objectives_engine.tsv  stage M2: the mod's four objectives engine folders into Objectives/Engine/
├── s1_global_renames.tsv     stage S1: top-level folder and tool package renames, archived records
├── s2_apps_and_deploy.tsv    stage S2: website crates to apps/ and legacy/, snake_case packages, deploy/
├── s2_brief_archive.tsv      stage S2: the executed S1 agent briefs into the archive
├── s2_caddy_folder.tsv       stage S2: the Caddyfile into deploy/caddy/, the one folder the Caddy container mounts
└── s2_crate_births.tsv       stage S2: the API's URL guard and the worker's cache policy become crates/
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
ticket record whose status is shipped or cancelled gets only its `spec`, `plan` and `owns` entries
rewritten. A file takes the treatment of the place it lands, so a file moved into the archive is
frozen from that move on.
Binary files and Git LFS pointers move with their folders and are never edited. SQL migrations (a
`.sql` file directly in a `migrations` folder, whose checksum `sqlx` pins once a database applies
it) move byte-identical and are never edited or verified.

### Running a stage manifest

1. `cargo xtask refactor relocate --manifest <path.tsv> --dry-run` prints, per row, the tracked
   files moved and the references rewritten by file kind, then every unresolved literal, and runs
   the verification below over the tree the plan would leave; it writes nothing and exits 1 on
   any unresolved literal or finding.
2. `cargo xtask refactor relocate --manifest <path.tsv> --apply` refuses, writing nothing, unless
   that dry run passes; otherwise it makes the moves and rewrites and then verifies the manifest.
3. `cargo xtask refactor relocate --verify` judges every committed manifest in this folder except
   `example.tsv`: no live tracked file spells a `path` row's retired `from` on segment boundaries
   (frozen records and the manifests themselves excluded), and no `rust_path` prefix is left in its
   scope. Exit 0 pass, 1 findings, 2 did not run.

A stage commits its manifest here with the moves it made, named after the stage (for example
`s1_global_renames.tsv`), and never edits it afterwards.

## Code

- [Relocation](/tools/xtask/src/commands/refactor/relocate/) — the parser, the passes, the moves
  and the verification that read these files.

## Boundaries

- Depends on: the relocation tool's manifest parser, which defines the format above.
- Used by: `cargo xtask refactor relocate --verify`, which judges every stage manifest here; the
  relocation tests, which run `example.tsv` on a throwaway checkout.
- Rules: `example.tsv` stays the format sample with one row of each kind
  (`relocate_example_manifest_parses_and_applies`); a committed stage manifest is never edited,
  since its rows are the retired spellings the verification keeps out of the tree.

## Related documentation

- [Laws and gates](/documentation/restructure/laws_and_gates.md) — the relocation law the
  verification enforces between stages.
- [Path coupling research](/documentation/archive/restructure_research/02_path_coupling.md) —
  every kind of path reference a move breaks.
