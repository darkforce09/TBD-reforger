**Status:** live

# S1 agent R1: global renames

You are agent R1, who runs the stage S1 global renames of the workspace restructure program with the relocation tool.

Read [the shared brief](/documentation_v2/restructure/agent_briefs/shared_brief.md) first (Rules, Efficiency, Spec, Report format), where
`<scratch>` = `<scratch>`.
Stage S0 is committed; the tree is clean when you start. You run alone: nobody else edits the tree
until you report.

## Goal
`assets_v2` → `assets`, `contracts_v2` → `contracts`, `documentation_v2` → `documentation`,
`tools_v2` → `tools` with snake_case tool folders and package names
(`verification_core`, `developer_tools`, `ticket_engine`; `xtask` unchanged); the documentation
mirror folders follow; the finished refactor program records and the superseded improved-layout
plans move to the archive. Amendment A1 (orchestrator): the ticket engine is NOT parked in a
`legacy/` folder in S1; it becomes `tools/ticket_engine` and is dissolved in S11.

## Steps
1. Copy the manifest draft at the end of this document into the manifests folder as s1_global_renames.tsv
   (the manifest is committed with the stage). Read `tools_v2/xtask/src/commands/refactor/README.md`
   and `relocate/README.md` to learn the tool's exact semantics (row order, nested rows, frozen
   areas, ticket TOMLs) and adjust the manifest if the draft does not fit them — for example if
   nested rows must be ordered differently. Keep every intent of the draft.
2. `cargo xtask refactor relocate --manifest documentation_v2/restructure/manifests/s1_global_renames.tsv --dry-run`
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
   pieces (`format!("{}_v2", …)`, `join("tools_v2")` split literals), `include!` targets that moved.
   Use `git grep -n -E "assets_v2|contracts_v2|documentation_v2|tools_v2|verification-core|developer-tools|ticket-engine"`
   outside `documentation/archive/`, `documentation/tickets/` and `.ai/tickets/` and list every
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

## Manifest draft

```text
# Stage S1 — global renames. Paths name the tree before the moves.
kind	from	to	scope
# documentation mirror folders first (more specific before their parents)
path	documentation_v2/assets_v2	documentation/assets
path	documentation_v2/contracts_v2	documentation/contracts
path	documentation_v2/tools_v2/developer-tools	documentation/tools/developer_tools
path	documentation_v2/tools_v2/ticket-engine	documentation/tools/ticket_engine
path	documentation_v2/tools_v2	documentation/tools
# finished program records and the superseded improved-layout plans go to the archive
path	documentation_v2/refactor_followup_tickets.md	documentation/archive/refactor_v2/refactor_followup_tickets.md
path	documentation_v2/refactor_move_manifest	documentation/archive/refactor_v2/refactor_move_manifest
path	documentation_v2/refactor_move_manifest.tsv	documentation/archive/refactor_v2/refactor_move_manifest.tsv
path	documentation_v2/refactor_orphan_spec_links.tsv	documentation/archive/refactor_v2/refactor_orphan_spec_links.tsv
path	documentation_v2/refactor_phase4_slices.tsv	documentation/archive/refactor_v2/refactor_phase4_slices.tsv
path	documentation_v2/refactor_phase5_slices.tsv	documentation/archive/refactor_v2/refactor_phase5_slices.tsv
path	documentation_v2/refactor_pin_catalogue.md	documentation/archive/refactor_v2/refactor_pin_catalogue.md
path	documentation_v2/refactor_program_plan.md	documentation/archive/refactor_v2/refactor_program_plan.md
path	documentation_v2/refactor_progress_checkpoint.md	documentation/archive/refactor_v2/refactor_progress_checkpoint.md
path	documentation_v2/refactor_style_lock.md	documentation/archive/refactor_v2/refactor_style_lock.md
path	documentation_v2/refactor_ticket_rewrites.tsv	documentation/archive/refactor_v2/refactor_ticket_rewrites.tsv
path	documentation_v2/refactor_writing_brief.md	documentation/archive/refactor_v2/refactor_writing_brief.md
path	documentation_v2/website/improved_layout	documentation/archive/improved_layout/website
path	documentation_v2/mod/improved_layout	documentation/archive/improved_layout/mod
path	apps/website/improved_layout/README.md	documentation/archive/improved_layout/website_code_tree_anchor.md
path	apps/mod/improved_layout/README.md	documentation/archive/improved_layout/mod_code_tree_anchor.md
# top-level folders
path	documentation_v2	documentation
path	assets_v2	assets
path	contracts_v2	contracts
path	tools_v2/verification-core	tools/verification_core
path	tools_v2/developer-tools	tools/developer_tools
path	tools_v2/ticket-engine	tools/ticket_engine
path	tools_v2	tools
# package names of the tool crates
text	verification-core	verification_core
text	developer-tools	developer_tools
text	ticket-engine	ticket_engine
```
