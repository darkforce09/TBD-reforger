# Relocation

The body of `cargo xtask refactor relocate`: it reads a relocation manifest, computes every move and
every reference rewrite the manifest asks for before anything is written, verifies in memory that
the tree the plan would leave spells no retired path or Rust prefix in a live tracked file, applies
the plan in one pass, and verifies the checkout again afterwards.

## Contents

```text
tools/xtask/src/commands/refactor/relocate/
├── file_treatment.rs     what a file may receive: live, frozen record, closed ticket, excluded
├── manifest.rs           the manifest parser, refusing the whole manifest on any bad row
├── mod.rs                the three modes (dry run, apply, verify) and their exit codes
├── path_mapping.rs       where each path lands after the moves, and the relative-path math
├── path_references/      the `path` row pass: repository-root spellings and relative literals
├── plan_application.rs   the `git mv` moves and the file writes, undone on any failure
├── plan_summary.rs       the per-row summary a dry run and an apply print
├── planned_tree.rs       the tree a plan would leave, read in memory for the verification
├── relocation_plan.rs    the plan: checked moves, rewritten files, unresolved literals
├── repository_files.rs   the tracked files, which are text, the crate each sits in, the judged tree
├── retired_spellings.rs  the verification: no retired `from` left in a live file or scope
├── rust_lexer.rs         a lossless Rust tokenizer telling code, literals and comments apart
├── rust_paths/           the `rust_path` row pass: `use` trees, code paths, doc links, chains
├── tests/                unit tests and whole runs on throwaway git checkouts
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
                        file_treatment.rs                   ▼
                                                 retired_spellings.rs
                                                 (--verify, end of --apply)
```

`repository_files.rs` lists the index (`git ls-files -z`) and asks `git check-attr` which files
carry a non-text attribute; a file is edited only when no attribute marks it binary, it holds no
NUL in its first 8000 bytes, it is UTF-8 and it is not a Git LFS pointer. Binaries and pointers
move with their folders unread.

`file_treatment.rs` sorts each file before any pass runs. Live files take every rewrite. Markdown
under the archive and the ticket documents takes link destination rewrites only; other files there,
the `.tsv` manifests of the manifests folder (their `from` columns name retired paths on purpose),
the manifest being run and every SQL migration (a `.sql` file directly in a `migrations` folder,
the files `sqlx` reads and pins by checksum once applied) take none; the rest of the manifests
folder, its README included, is live, and so is every other `.sql` file (a seed, a file in a
subfolder of a `migrations` folder) and every other file of a `migrations` folder. A ticket record whose `status` the ticket engine calls shipped or cancelled takes rewrites on
its `spec`, `plan` and `owns` values only (a multi-line `owns` array through its closing line), the
fields the ticket engine resolves against the tree and the wave lock; an open ticket is live. A
file is treated by where it lies after the moves, with the areas moved where the manifest puts
them, exactly as the verification treats it: a file a row moves into the archive is frozen (its
prose keeps quoting its history), one a row moves out of it is live, and the areas are found
whether this build of the tool names them as they lie before or after the moves.

`path_mapping.rs` turns the `path` rows into one mapping that relocates a path by its longest moved
prefix, so nested rows compose. `relocation_plan.rs` runs the three passes, each over the text the
previous one produced, refuses the whole plan when a `from` is not tracked or a `to` exists, and
records every literal it could not rewrite.

`planned_tree.rs` presents the checkout as the plan would leave it, in memory: every tracked path
relocated, every rewritten file holding its planned text, every other file read where it lies.
`--dry-run` prints the plan and then runs `retired_spellings.rs` over that planned tree, the same
verification `--verify` runs after an apply, listing every finding as `path:line` at its new path;
it exits 1 when anything is unresolved or the planned tree keeps a retired spelling. `--apply` runs
the same two checks first and writes nothing unless both are clean; then `plan_application.rs` runs
the moves shallowest first, writes every rewritten file at its new path, undoes everything on a
failure, and `retired_spellings.rs` judges the same manifest on the checkout. A plan that passes
its dry run therefore leaves a checkout that verifies clean, whatever spelling a pass missed: the
miss shows in the dry run instead of after the apply.

The verification reads the same files and the same allowed spans as the passes
(`path_references::allowed_spans`, `file_treatment.rs`), with the frozen areas moved where the
manifest's own moves put them, and classifies each occurrence with the same
`path_tokens::classify_occurrence`. A spelling of a retired `from` that, read with the segments
before it or with the escape letter glued to it, names a path that exists now is another path and
no finding.

## Public surface

- `dry_run(root, manifest)`, `apply(root, manifest)` and `verify(root, Option<manifest>)` in
  `mod.rs`, called by `tools/xtask/src/commands/refactor/dispatch.rs`; each returns the exit
  code (0 clean, 1 findings, 2 did not run).
- `EXAMPLE_MANIFEST`: the format sample's file name, never judged as a stage manifest.

## Boundaries

- Depends on: `verification_core` (`Verdict`, `Report`, `NotRun`); `process_runner::Run` for git;
  `ticket_engine::StatusName` for the closed ticket statuses; the frozen-area constants in
  `tools/xtask/src/core/repository_layout.rs`; `git` on the path.
- Used by: `tools/xtask/src/commands/refactor/dispatch.rs` only.
- Rules: nothing is written before the whole plan is computed, free of unresolved items
  (`relocate_unresolvable_literal_fails_apply_with_nothing_written`) and its planned tree verifies
  clean (`relocate_dry_run_verifies_the_planned_tree_and_fails_on_a_hidden_leftover`); a SQL
  migration moves byte-identical and is never judged, while every other `.sql` file is live
  (`relocate_sql_migrations_move_byte_identical_and_are_not_verified`); a spelling that only looks
  like a path is never rewritten: a literal of separators alone and a plain fixture path that only
  starts with a moved folder's name stay as written
  (`relocate_lone_separator_literals_name_no_path`,
  `relocate_plain_fixture_paths_under_a_moved_folder_name_stay_as_written`); the passes
  and the verification share `file_treatment.rs`, `path_references::allowed_spans` and
  `path_tokens::classify_occurrence`, so they judge the same bytes the same way; tests are named
  `relocate_*` and run on throwaway checkouts, never on this one.

## Related documentation

- [Relocation manifests](/documentation/restructure/manifests/README.md) — the manifest format
  and the stage manifests.
- [Path coupling research](/documentation/archive/restructure_research/02_path_coupling.md) —
  every kind of path reference a move breaks.
