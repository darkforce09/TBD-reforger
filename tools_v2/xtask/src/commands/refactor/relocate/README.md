# Relocation

The body of `cargo xtask refactor relocate`: it reads a relocation manifest, computes every move and
every reference rewrite the manifest asks for before anything is written, applies them in one pass,
and verifies afterwards that no live tracked file still spells a retired path or Rust prefix.

## Contents

```text
tools_v2/xtask/src/commands/refactor/relocate/
├── file_treatment.rs     what a file may receive: live, frozen record, closed ticket, excluded
├── manifest.rs           the manifest parser, refusing the whole manifest on any bad row
├── mod.rs                the three modes (dry run, apply, verify) and their exit codes
├── path_mapping.rs       where each path lands after the moves, and the relative-path math
├── path_references/      the `path` row pass: repository-root spellings and relative literals
├── plan_application.rs   the `git mv` moves and the file writes, undone on any failure
├── plan_summary.rs       the per-row summary a dry run and an apply print
├── relocation_plan.rs    the plan: checked moves, rewritten files, unresolved literals
├── repository_files.rs   the tracked files, which are text, and the crate each sits in
├── retired_spellings.rs  the verification: no retired `from` left in a live file or scope
├── rust_lexer.rs         a lossless Rust tokenizer telling code, literals and comments apart
├── rust_paths/           the `rust_path` row pass: `use` trees, code paths, doc links, chains
├── tests/                unit tests and whole runs on throwaway git checkouts
├── text_edits.rs         byte-span edits, their merge and application, and the allowed spans
└── text_tokens.rs        the `text` row pass: identifier-like tokens on word boundaries
```

## How it works

```text
manifest.rs ─rows─▶ relocation_plan.rs ─plan─▶ plan_summary.rs      (--dry-run, --apply)
                     │ per tracked text file:   └─▶ plan_application.rs (--apply)
                     │  1. path_references/                │
                     │  2. rust_paths/ (live .rs)          ▼
                     │  3. text_tokens.rs (live)     retired_spellings.rs
                     └─ repository_files.rs,         (--verify, end of --apply)
                        file_treatment.rs
```

`repository_files.rs` lists the index (`git ls-files -z`) and asks `git check-attr` which files
carry a non-text attribute; a file is edited only when no attribute marks it binary, it holds no
NUL in its first 8000 bytes, it is UTF-8 and it is not a Git LFS pointer. Binaries and pointers
move with their folders unread.

`file_treatment.rs` sorts each file before any pass runs. Live files take every rewrite. Markdown
under the archive and the ticket documents takes link destination rewrites only; other files there,
the manifests folder and the manifest being run take none. A ticket record whose `status` the
ticket engine calls shipped or cancelled takes rewrites on its `spec` and `plan` lines only; an open
ticket is live.

`path_mapping.rs` turns the `path` rows into one mapping that relocates a path by its longest moved
prefix, so nested rows compose. `relocation_plan.rs` runs the three passes, each over the text the
previous one produced, refuses the whole plan when a `from` is not tracked or a `to` exists, and
records every literal it could not rewrite. `--apply` never writes a plan with an unresolved item;
otherwise `plan_application.rs` runs the moves shallowest first, writes every rewritten file at its
new path, undoes everything on a failure, and `retired_spellings.rs` judges the same manifest.

The verification reads the same files and the same allowed spans as the passes, with the frozen
areas moved where the manifest's own moves put them, so a clean apply always verifies clean. A
spelling of a retired `from` that, read with the segments before it, names a path that exists now
is another path and no finding.

## Public surface

- `dry_run(root, manifest)`, `apply(root, manifest)` and `verify(root, Option<manifest>)` in
  `mod.rs`, called by `tools_v2/xtask/src/commands/refactor/dispatch.rs`; each returns the exit
  code (0 clean, 1 findings, 2 did not run).
- `EXAMPLE_MANIFEST`: the format sample's file name, never judged as a stage manifest.

## Boundaries

- Depends on: `verification_core` (`Verdict`, `Report`, `NotRun`, `proc::Run` for git);
  `ticket_engine::StatusName` for the closed ticket statuses; the frozen-area constants in
  `tools_v2/xtask/src/core/repository_layout.rs`; `git` on the path.
- Used by: `tools_v2/xtask/src/commands/refactor/dispatch.rs` only.
- Rules: nothing is written before the whole plan is computed and free of unresolved items
  (`relocate_unresolvable_literal_fails_apply_with_nothing_written`); the passes and the
  verification share `file_treatment.rs` and `path_references::allowed_spans`, so they judge the
  same bytes; tests are named `relocate_*` and run on throwaway checkouts, never on this one.

## Related documentation

- [Relocation manifests](/documentation_v2/restructure/manifests/README.md) — the manifest format
  and the stage manifests.
- [Path coupling research](/documentation_v2/archive/restructure_research/02_path_coupling.md) —
  every kind of path reference a move breaks.
