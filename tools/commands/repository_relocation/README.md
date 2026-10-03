# Repository relocation

The `repository_relocation` crate: moves tracked files and folders and rewrites every reference to
them from a relocation manifest, so no path is ever rewritten by hand, and proves afterwards that
no live file still spells a path or Rust prefix a manifest retired. It is the work behind
`cargo xtask refactor relocate`, which the restructure program's stages run for every move.

## Contents

```text
tools/commands/repository_relocation/
├── Cargo.toml  the `repository_relocation` library package: aho-corasick, verification_core, process_runner, repository_layout, ticket_model; layout tier 3
└── src/        the manifest parser, the plan, the rewrite passes, the moves and the verification
```

## How it works

```text
xtask refactor relocate ──▶ dry_run / apply / verify (root, manifest) ──▶ exit code 0, 1 or 2
                                │
   manifest rows ─▶ plan (moves, path, rust_path and text rewrites) ─▶ planned tree verified
                                │                                          │
                                └── apply: git mv + writes, undone on failure ─▶ checkout verified
```

The xtask binary parses the flags, finds the checkout root and calls one of the three modes.
`dry_run` prints the plan of a manifest and verifies, in memory, the tree it would leave; `apply`
refuses unless that dry run is clean, then makes the moves and writes and verifies the checkout;
`verify` judges one manifest, or every committed manifest in
`documentation/restructure/manifests/` except the format sample `example.tsv`, against the
checkout as it stands; each committed manifest's scopes follow the moves of the manifests that
entered the history after it, since a committed manifest is never edited. A climbing token
`<seg>/../…` counts as a path reference only under an anchor (the file's folder, the owning
crate's folder, the repository root) where its named lead (the segments before its first `..`) is a tracked folder, so a test
datum such as `"7/../.."` is never read as a path and never stops an apply. A literal read from
the owning crate's folder (a `CARGO_MANIFEST_DIR` join) follows a moved file to the crate the planned
tree gives it: the nearest folder holding a crate manifest after the moves, counting one a row
moves there and an untracked one on disk; where no folder below the repository root holds one, the
literal is unresolved and the apply refuses, never re-anchoring it at the root.

The verification is a single pass over the tree, however many manifests it judges: every text file
is read once, and one Aho-Corasick automaton over every manifest's retired `path` spellings and
`rust_path` prefix segments scans each file once; each hit is then judged for every row that
retires it, under that row's manifest's file treatment and within that row's scope, so each
manifest gets exactly the findings it gets judged alone. `src/README.md` describes the passes, the
file treatments, the move order, that composition and the single pass.

## Getting started

Run from the repository root:

```bash
cargo test -p repository_relocation   # unit tests and whole runs on throwaway git checkouts
cargo xtask refactor relocate --verify   # judge every committed stage manifest
```

## Configuration

No feature and no environment variable; `git` must be on the path.

## Public surface

- At the crate root and in `prelude`: `dry_run(root, manifest)`, `apply(root, manifest)` and
  `verify(root, Option<manifest>)`, each returning the process exit code (0 clean, 1 findings,
  2 did not run).

## Boundaries

- Depends on: `aho-corasick` (the verification's combined matcher), `verification_core`
  (verdicts, the run report, `NotRun`), `process_runner` (git,
  including the history of the manifests folder),
  `repository_layout` (the frozen areas and the manifests folder), `ticket_model` (the closed
  ticket statuses).
- Used by: the `refactor` command group of the xtask binary
  (`tools/xtask/src/commands/refactor/dispatch.rs`).
- Rules: tier 3 of `tools/commands` (`cargo xtask verify crate-tiers`); nothing is written before
  the whole plan is computed and its planned tree verifies clean; a failed apply leaves the index
  and the working tree byte-identical; a climbing token whose named lead is no tracked folder
  at an anchor is not read from that anchor
  (`relocate_climb_whose_lead_names_no_folder_stays_as_written`); a crate-folder literal of a file
  moved where no crate manifest lies below the root is unresolved
  (`relocate_manifest_dir_joins_into_a_folder_with_no_crate_manifest_are_unresolved`); tests run on throwaway checkouts
  and read only the committed format sample from this one.

## Related documentation

- [Relocation manifests](/documentation/restructure/manifests/README.md) — the manifest format
  and the stage manifests.
- [Laws and gates](/documentation/restructure/laws_and_gates.md) — the relocation law the
  verification enforces between stages.
