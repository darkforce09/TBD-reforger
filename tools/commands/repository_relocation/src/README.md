# Repository relocation source

The body of `cargo xtask refactor relocate`: it reads a relocation manifest, computes every move and
every reference rewrite the manifest asks for before anything is written, verifies in memory that
the tree the plan would leave spells no retired path or Rust prefix in a live tracked file, applies
the plan in one pass, and verifies the checkout again afterwards.

## Contents

```text
tools/commands/repository_relocation/src/
├── file_treatment.rs     what a file may receive: live, frozen record or index, fixture, excluded
├── lib.rs                the crate root: module header, `mod` lines and the re-exported modes
├── manifest.rs           the manifest parser, refusing the whole manifest on any bad row
├── manifest_chronology.rs  the stage manifests in the order they entered the history, renames followed
├── move_placement.rs     where the moves land, the rows that collide, the order the moves run in
├── path_mapping.rs       where each path lands after the moves, and the relative-path math
├── path_references/      the `path` row pass: repository-root spellings and relative literals
├── plan_application.rs   the `git mv` moves and the file writes, undone byte-identically on any failure
├── plan_summary.rs       the per-row summary a dry run and an apply print
├── planned_tree.rs       the tree a plan would leave, read in memory for the verification
├── prelude.rs            `dry_run`, `apply` and `verify` for glob import
├── relocation_modes.rs   the three modes (dry run, apply, verify) and their exit codes
├── relocation_plan.rs    the plan: checked moves, rewritten files, unresolved and ambiguous literals
├── repository_files.rs   the tracked files, which are text, the crate each sits in, the judged tree
├── retired_spellings.rs  the verification: no retired `from` left in a live file or scope
├── retired_spellings/   the verification's read-once tree text and its combined spelling matcher
├── rust_lexer.rs         a lossless Rust tokenizer telling code, literals and comments apart
├── rust_paths/           the `rust_path` row pass: `use` trees, code paths, doc links, chains
├── scope_history.rs      where a manifest's scope lies, and where its retired spellings live again, after the later manifests' moves
├── text_edits.rs         byte-span edits, their merge and application, and the allowed spans
└── text_tokens.rs        the `text` row pass: identifier-like tokens on word boundaries
```

## How it works

```text
manifest.rs ─rows─▶ relocation_plan.rs ─plan─▶ plan_summary.rs           (--dry-run, --apply)
                     │ per tracked text file:   ├─▶ planned_tree.rs ─▶ retired_spellings.rs
                     │  1. path_references/     │   (--dry-run, --apply before any write)
                     │  2. rust_paths/ (live .rs)
                     │  3. text_tokens.rs (live)└─▶ plan_application.rs (--apply, clean plan)
                     └─ repository_files.rs,                │
                        file_treatment.rs,                  ▼
                        move_placement.rs        retired_spellings.rs
                                                 (--verify, end of --apply)
```

`repository_files.rs` lists the index (`git ls-files -z`) and asks `git check-attr` which files
carry a non-text attribute; a file is edited only when no attribute marks it binary, it holds no
NUL in its first 8000 bytes, it is UTF-8 and it is not a Git LFS pointer. Binaries and pointers
move with their folders unread.

`file_treatment.rs` sorts each file before any pass runs. Live files take every rewrite. Markdown
under the archive and the ticket documents takes link destination rewrites only, except that a
`README.md` there is a live index of its folder: the tree part of its Contents block's lines (the
root folder and each entry's name, up to the two spaces before the role) takes every rewrite and is
verified as a live file is, while its prose and roles stay as written. This crate's own test
sources (the `RELOCATION_TEST_SOURCES` folder) spell paths of throwaway checkouts, so only their code takes the `rust_path`
and `text` rewrites and is verified, never a string literal or a comment, and no `path` row reaches
them. Other files of the frozen areas,
the `.tsv` manifests of the manifests folder (their `from` columns name retired paths on purpose),
the manifest being run and every SQL migration (a `.sql` file directly in a `migrations` folder,
the files `sqlx` reads and pins by checksum once applied) take none; the rest of the manifests
folder, its README included, is live, and so is every other `.sql` file (a seed, a file in a
subfolder of a `migrations` folder) and every other file of a `migrations` folder. The legacy
ticket data folder (`.ai/tickets`, records awaiting import into the central ticket manager) takes
none. A
file is treated by where it lies after the moves, with the areas moved where the manifest puts
them, exactly as the verification treats it: a file a row moves into the archive is frozen (its
prose keeps quoting its history), one a row moves out of it is live, and the areas are found
whether this build of the tool names them as they lie before or after the moves.

`path_mapping.rs` turns the `path` rows into one mapping that relocates a path by its longest moved
prefix, so nested rows compose. `relocation_plan.rs` runs the three passes, each over the text the
previous one produced, refuses the whole plan when a `from` is not tracked or a `to` exists, and
records every literal it could not rewrite and every literal it left as written as ambiguous (a
relative literal its spelling does not pin to one anchor; see `path_references/`). Ambiguous
literals are printed with `path:line` for review and never stop a run. The crate folders that anchor
`CARGO_MANIFEST_DIR`-relative literals are the tracked crate manifests plus the untracked ones on
disk (crates being born), the same set before and after the moves, so a literal is rewritten only
when a row moves what it names or the file holding it. After the moves a file's owning crate is
read from the planned tree: the nearest folder holding a crate manifest once the rows have moved
every path, so a manifest a row moves into the destination, or an untracked one already there,
owns the files moved below it. When no folder below the repository root holds one (the root
manifest is the workspace's and builds no package), a literal read from the crate folder is
unresolved and the apply refuses; it is never re-anchored at the root.

`move_placement.rs` proves that the mapping's tree is one `git mv` can make, and refuses the plan
otherwise, naming both manifest lines of each conflict: two rows with the same `to`, a `to` inside
its own `from`, two files landing on one path or a file landing where another needs a folder, and
a file landing inside another row's `to` unless it comes from that row's `from` or from a row whose
`to` lies strictly inside it. Folder rows that only share a destination parent, such as a folder
that becomes `crate/src` and two folders that become `crate/src/ortho` and `crate/src/orbit`, are
no conflict. It then orders the moves: a move runs after every move whose `to` holds its `to` (that
one needs its `to` absent, or `git mv` would nest the folder inside it) and after every move whose
`from` holds its `to` (that one frees the place); otherwise shallower `from` first, then manifest
order. Rows no order satisfies, such as two folders that swap names, are refused. Last, it replays
the moves in that order over the tracked paths and refuses, with the message the apply itself would
stop on, a move whose source is gone or whose `to` is already occupied when it runs, such as a file
row whose folder row has already carried the file to that `to`; so a dry run refuses whatever the
apply would refuse.

`planned_tree.rs` presents the checkout as the plan would leave it, in memory: every tracked path
relocated, every rewritten file holding its planned text, every other file read where it lies.
`--dry-run` prints the plan and then runs `retired_spellings.rs` over that planned tree, the same
verification `--verify` runs after an apply, listing every finding as `path:line` at its new path;
it exits 1 when anything is unresolved or the planned tree keeps a retired spelling. `--apply` runs
the same two checks first and writes nothing unless both are clean; then `plan_application.rs` runs
the moves in that order, each into a `to` it checks is absent, writes every rewritten file at its
new path, and `retired_spellings.rs` judges the same manifest on the checkout. Every step is
journaled; a failure at any step restores the rewritten files' original bytes, renames the moves
back last first, removes the folders the apply created and writes back the index file copied
before the first move, so the index and the working tree are byte-identical to before. A plan that passes
its dry run therefore leaves a checkout that verifies clean, whatever spelling a pass missed: the
miss shows in the dry run instead of after the apply.

The verification reads the same files and the same allowed spans as the passes
(`path_references::allowed_spans`, `file_treatment.rs`), with the frozen areas moved where the
manifest's own moves put them, and classifies each occurrence with the same
`path_tokens::classify_occurrence`. A spelling of a retired `from` that, read with the segments
before it or with the escape letter glued to it, names a path that exists now is another path and
no finding.

The verification is one pass however many manifests it judges (`retired_spellings/`): the tree's
text files are read once, every manifest's `path` spellings and `rust_path` prefix segments go into
one Aho-Corasick automaton that scans each file once, and each hit is judged for every row that
retires it under that row's manifest's treatment of the file and within that row's scope. A file's
treatment is computed once per distinct set of moved frozen areas, its allowed spans once per
treatment and only when it holds a hit, and the Rust path pass runs on a file only for a prefix
whose every segment the file spells. Each manifest gets the verdicts, notes and offence order it
gets judged alone; the per-row judge this replaces is kept as the oracle of
`relocate_verify_single_pass_matches_the_per_row_judge_over_composed_manifests`.

`--verify` with no manifest judges every stage manifest in the order `manifest_chronology.rs`
reads from the history: the first commit that added each one under any path it has had (one
`git log --find-renames=100% --diff-filter=AR` over every `.tsv` path, each exact rename carrying
the file's place to its new path, and one `git diff --cached` against `HEAD` for a staged move),
manifests one commit added by name, uncommitted ones last by name; a shallow clone is a did-not-run.
Moving the manifests folder therefore keeps the order. A committed manifest is never edited, so
`scope_history.rs` follows each `rust_path` row's folder or glob scope through its own manifest's
`path` rows and then those of every manifest after it: a scope a row moved is judged at its new
folder, a scope a row emptied, its own manifest's rows file by file or a later manifest's, holds
with a `note:` line naming the first such row, and a missing scope no row explains is a
did-not-run. The files a row took out of a scope are not judged at their destinations: a
`crate::` prefix means another crate's module once its file has left the crate. A `path` row's
retired spelling (its `from` and every path below it) is legal again only at or below the `to` of a
later manifest's `path` row whose `to` is that `from` or a path below it, never a folder above it
(`LaterMoves::revivals_of`): there its occurrences are no offence of the row, files tracked there
do not keep its `from` tracked, and each revival prints a `note:` line; a row never revives its own
manifest's spellings, and a still-later manifest that retires the spelling again judges it with its
own row. `text` rows are not judged.

## Public surface

- `dry_run(root, manifest)`, `apply(root, manifest)` and `verify(root, Option<manifest>)` in
  `relocation_modes.rs`, re-exported at the crate root and in `prelude.rs` and called by
  `tools/xtask/src/commands/refactor/dispatch.rs`; each returns the exit code (0 clean, 1
  findings, 2 did not run).
- `EXAMPLE_MANIFEST` (crate-internal): the format sample's file name, never judged as a stage
  manifest.

## Boundaries

- Depends on: `verification_core` (`Verdict`, `Report`, `NotRun`); `process_runner::Run` for git;
  `aho-corasick` for the verification's combined matcher;
  the frozen-area constants in
  `tools/foundation/repository_layout/src/documentation_locations.rs` and its legacy ticket data
  folder; `git` on the path.
- Used by: `tools/xtask/src/commands/refactor/dispatch.rs` only.
- Rules: nothing is written before the whole plan is computed, free of unresolved items and its planned tree verifies
  clean; a SQL
  migration moves byte-identical and is never judged, while every other `.sql` file is live; a spelling that only looks
  like a path is never rewritten: a literal of separators alone and a plain fixture path that only
  starts with a moved folder's name stay as written; a literal every crate
  spells for its own files, a fixture path relative to a temporary checkout and a path whose tail
  names nothing stay as written in a file that leaves its crate, listed as ambiguous; each move lands
  exactly at its `to` in any manifest order, colliding rows are refused with
  both lines, and a failure after any
  step leaves the index and the working tree byte-identical, and the dry run refuses a row
  the apply would meet occupied;
  a crate whose manifest git does not track yet keeps its literals through an unrelated apply; a file moved into a new
  crate folder re-anchors its `CARGO_MANIFEST_DIR` joins at the crate manifest the same manifest
  moves there or an untracked one on disk, and a file moved where no crate manifest lies below the
  root leaves them unresolved; an earlier manifest's
  scope is judged where later manifests moved it, holds when they emptied it and fails closed
  otherwise, and a
  scope its own rows emptied file by file applies and holds; a retired
  `path` spelling is legal again only below a later manifest's `to`, never from an earlier one,
  and a later retirement judges it again; the
  manifest order survives a move of the manifests folder; a frozen area's README
  index takes live rewrites in its Contents tree only; this crate's test sources keep
  their literals and comments; the passes
  and the verification share `file_treatment.rs`, `path_references::allowed_spans` and
  `path_tokens::classify_occurrence`, so they judge the same bytes the same way.

## Related documentation

- [Relocation manifests](/documentation/relocation_manifests/README.md) — the manifest format
  and the stage manifests.
- [Path coupling research](/documentation/archive/restructure_research/02_path_coupling.md) —
  every kind of path reference a move breaks.
