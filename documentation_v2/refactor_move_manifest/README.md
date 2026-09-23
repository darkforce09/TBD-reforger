**Status:** live — Documentation V2 move manifest summary

# Documentation V2 move manifest summary

The operator's reading copy of [refactor_move_manifest.tsv](/documentation_v2/refactor_move_manifest.tsv),
the row-by-row plan of the documentation move: its counts, its conventions, the ordering P2-1
follows, and the checkpoint CP1 questions with the operator's answers.

## Contents

```text
documentation_v2/refactor_move_manifest/
├── cp1_questions.md                   numbered questions for checkpoint CP1, with options, recommendations and answers
├── live_targets.md                    every live target with its writer, primary source and pending_merge sources
└── archive_and_reference_sets.md      archive topics with their files, reference sets, frozen corpus, evidence
```

## How it works

The TSV holds 1071 rows, one per source file: every tracked file under `docs/` and
`documentation_v2/` (the `refactor_*` program files excepted), every non-README markdown file under
`apps/`, `tools_v2/`, `contracts_v2/` and `assets_v2/`, and the three `.ai/artifacts/` spec files
tickets cite. Columns: `source`, `target`, `action`, `class`, `writer`, `note`; `-` marks an empty
cell. P0-2 wrote the mechanical rows and P0-3 the judgment rows; P0-5 merged them, applied the
operator's checkpoint CP1 answers and checked:

- **Coverage.** Every file of that set appears exactly once as a source.
- **Targets.** No two rows share a target, except a `collapse-duplicate` row (its target is the
  kept row's target) and the `delete` and `done-by-P1-2` rows (target `-`). A `merge-into` row
  targets its own `documentation_v2/pending_merge/<writer>/` file and its note is the final
  target. `done-by-P1-1` rows keep the archive path P1-1 moves the file to, so the link
  rewriters resolve through them.
- **Names.** Targets are snake_case, except `README.md`, `t-<id>_plan.md`, code-spelled mirror
  folders (`map-engine`, `graphics-engine`, `tbd-framework`, `tbd-export`, `tbd-emcp`, `api_v2`,
  the tools_v2 crate names, `Scripts/WorkbenchGame/EquipmentVehicleExport`, `UI`), hyphenated
  evidence folders and JSON under a `verification_evidence/` folder, and everything under
  `pending_merge/`, which keeps original names (a `README.md` source becomes its whole path in
  lower case with `/` spelled `__`; a `page.md` becomes `<area>__<page>__page.md`). Archived
  redirect stubs are named the same way; their note names the real target and its final path.
- **Retired roots.** No target sits under `docs/`, and no row spells the archived wave-plan
  paths or the wave environment variables the fossil-path guard bans.
- **Ordering.** The 12 rows whose note starts `ORDER:1` or `ORDER:2` swap a draft
  out of a tracked path before a primary moves in (below).
- **Near duplicates.** The 5 `collapse-duplicate` rows whose note starts `NEAR-DUPLICATE:`
  differ from their kept file: three by a final newline, one by a two-line header, one by
  both; each note gives the exact bytes. Every other collapse is byte-identical to its kept
  file.

## Counts

### By action

| action | rows |
|---|---|
| move | 589 |
| move+rename | 205 |
| merge-into | 102 |
| archive | 77 |
| collapse-duplicate | 52 |
| delete | 33 |
| done-by-P1-1 | 7 |
| done-by-P1-2 | 6 |
| **total** | **1071** |

### By class

| class | rows |
|---|---|
| frozen | 494 |
| data | 222 |
| live-rewrite | 219 |
| archive | 85 |
| evidence | 51 |
| **total** | **1071** |

### By writer

| writer | rows |
|---|---|
| - | 852 |
| F03 | 32 |
| F01 | 26 |
| F02 | 26 |
| F04 | 21 |
| F17 | 21 |
| F10 | 19 |
| F08 | 18 |
| F12 | 8 |
| F16 | 7 |
| F09 | 6 |
| F13 | 6 |
| F07 | 5 |
| F14 | 5 |
| F11 | 4 |
| F15 | 4 |
| F18 | 4 |
| P3-2 | 4 |
| F05 | 2 |
| G2 | 1 |
| **total** | **1071** |

### By target folder

| target folder | rows |
|---|---|
| documentation_v2/tickets/ | 494 |
| documentation_v2/mod/ | 175 |
| documentation_v2/website/ | 143 |
| documentation_v2/pending_merge/ | 102 |
| documentation_v2/archive/ | 85 |
| none (delete, done-by-P1-2) | 39 |
| documentation_v2/runbooks/ | 11 |
| documentation_v2/design_system/ | 9 |
| documentation_v2/standards/ | 5 |
| documentation_v2/known_bugs/ | 3 |
| .ai/ | 2 |
| documentation_v2/ (root file) | 1 |
| documentation_v2/contracts_v2/ | 1 |
| documentation_v2/tools_v2/ | 1 |
| **total** | **1071** |

## Ordering notes for P2-1

1. The six `ORDER:1` rows sit at the top of the TSV: each moves a draft out of a tracked path
   into `pending_merge/` before its `ORDER:2` row moves the primary in. Applying the rows in file
   order is therefore safe. The pairs:
   - `documentation_v2/README.md`: `documentation_v2/README.md` leaves, `docs/website/README.md` arrives.
   - `documentation_v2/design_system/design_tokens.md`: `documentation_v2/design_system/design_tokens.md` leaves, `docs/website/frontend/THEME.md` arrives.
   - `documentation_v2/mod/README.md`: `documentation_v2/mod/README.md` leaves, `docs/mod/README.md` arrives.
   - `documentation_v2/runbooks/local_development.md`: `documentation_v2/runbooks/local_development.md` leaves, `docs/website/DEV_RUNBOOK.md` arrives.
   - `documentation_v2/website/frontend/README.md`: `documentation_v2/website/frontend/README.md` leaves, `docs/website/frontend/ROADMAP.md` arrives.
   - `documentation_v2/website/frontend/apps/editor/README.md`: `documentation_v2/website/frontend/apps/editor/README.md` leaves, `docs/specs/Mission_Creator_Architecture/README.md` arrives.
2. `git mv` does not create folders: create each target's parent folder first.
3. Before each `collapse-duplicate` `git rm`, check that the file's sha256 equals the kept row's
   source; the five `NEAR-DUPLICATE` rows are the accepted exceptions (a plan amendment),
   and their notes state the difference.
4. The 33 `delete` rows are the boilerplate visual-reference READMEs (sha256 8f633b199849…);
   their facts go into the README the feature folder's writer adds to each set.
5. Skip the `done-by-P1-1` and `done-by-P1-2` rows: Phase 1 applied them. Check that their
   sources are gone and the P1-1 archive targets exist.
6. A `merge-into` note is the final target; strip an `ORDER:1 ` prefix before reading it.
7. Content stays untouched in commit 2a so git detects every move as a rename. P2-2's constants
   land in the same commit: `ticket sync` skips the ROADMAP marker and the gap-analysis column
   silently when their files are missing, the fossil-path guard needs
   `ARCHIVED_WAVE_PLAN_READERS` to name the moved files, and `verify api-readiness` needs the
   register path ([refactor_pin_catalogue.md](/documentation_v2/refactor_pin_catalogue.md)).
8. Two targets sit outside `documentation_v2/`: `.ai/factory_pack_wave` and
   `.ai/tickets/plan_template.md`. Afterwards no file remains under `docs/`; remove the folder.

## Boundaries

- Depends on: P0-2's mechanical rows, P0-3's judgment rows and P0-4's ticket plan (scratchpad
  inputs of the Documentation V2 program).
- Used by: P2-1 (moves), P2-2 (pins), P2-3 (tickets), P2-5 and P2-6 (links), the Phase 5 writers
  (pending_merge sources) and checkpoint CP1.
- Rules: the TSV is the authority and stays one file; this folder only summarises it.

## Related documentation

- [Program plan](/documentation_v2/refactor_program_plan.md)
- [Writing brief](/documentation_v2/refactor_writing_brief.md)
- [Ticket rewrites](/documentation_v2/refactor_ticket_rewrites.tsv) and
  [orphan spec links](/documentation_v2/refactor_orphan_spec_links.tsv)
- [Pin catalogue](/documentation_v2/refactor_pin_catalogue.md)

