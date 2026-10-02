**Status:** live

# S1 agent R1: global renames

You are agent R1, who runs the stage S1 global renames of the workspace restructure program with the relocation tool.

Read [the shared brief](/documentation/restructure/agent_briefs/shared_brief.md) first (Rules, Efficiency, Spec, Report format), where
`<scratch>` = `<scratch>`.
Stage S0 is committed; the tree is clean when you start. You run alone: nobody else edits the tree
until you report.

## Goal
The four top-level folders `assets`, `contracts`, `documentation` and `tools` drop their `_v2`
suffix, and the tool crates take snake_case folders and package names in place of their hyphenated
spellings (`verification_core`, `developer_tools`, `ticket_engine`; `xtask` unchanged); the documentation
mirror folders follow; the finished refactor program records and the superseded improved-layout
plans move to the archive. Amendment A1 (orchestrator): the ticket engine is NOT parked in a
`legacy/` folder in S1; it becomes `tools/ticket_engine` and is dissolved in S11.

## Steps
1. Write the stage manifest (see [Manifest](#manifest)) into the manifests folder as
   s1_global_renames.tsv (the manifest is committed with the stage). Read `tools/xtask/src/commands/refactor/README.md`
   and `relocate/README.md` to learn the tool's exact semantics (row order, nested rows, frozen
   areas, ticket TOMLs) and adjust the manifest if the draft does not fit them — for example if
   nested rows must be ordered differently. Keep every intent of the draft.
2. `cargo xtask refactor relocate --manifest documentation/restructure/manifests/s1_global_renames.tsv --dry-run`
   (log it). Read the summary: files moved, references per kind, unresolved literals. Unresolved
   literals must be zero; if not, understand each one and fix the manifest or report it.
3. `--apply` (log it). The tool verifies after applying.
4. `git lfs ls-files | wc -l` must still be 2013 and `git lfs fsck --pointers` must pass;
   `.gitattributes` must name `assets/terrains/**`. Tracked file count must still be 16226 plus
   the new manifest file (`git ls-files | wc -l`).
5. `cargo metadata --format-version 1 --locked > /dev/null` and `cargo check --workspace --all-targets --locked`
   (logs). Package renames change `Cargo.lock` package names only (cargo updates them; run once
   without `--locked` if needed and report the lock diff: only the three renamed packages and
   their dependents' references may change).
6. Fix what the tool could not see, but only compile-level breakage: paths built at run time from
   pieces (`format!("{}_v2", …)`, a `join` of an old folder name as a split literal), `include!`
   targets that moved. Use `git grep -n -E` for the four old top-level folder names and the three
   hyphenated package names outside `documentation/archive/`, `documentation/tickets/` and `.ai/tickets/` and list every
   remaining hit with a verdict (fixed, legitimate history, or left for R2–R4 with the reason).
   Do not fix test failures or documentation gates beyond compiling: R2, R3 and R4 own those.
7. `cargo xtask refactor relocate --verify` (log).

## Owned files
The whole tree for the mechanical apply; afterwards only the manifest and the compile-level fixes of
step 6. Never hand-edit a path the tool can rewrite.

## Report additions
Section 2 must include the dry-run and apply summary numbers, the LFS and file counts, the lock
diff, and the list from step 6 (path:line, verdict).

Budget: M (250k tokens). Stop there and report done and not done.

## Manifest

The stage manifest is committed as
[s1_global_renames.tsv](/documentation/restructure/manifests/s1_global_renames.tsv).
