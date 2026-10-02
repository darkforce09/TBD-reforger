**Status:** live

# T-1092 — Modularise, document and gate the mod scripts

Program ticket. Children: T-1092.1 (gates and mechanical sweep), T-1092.2 (shared foundations),
T-1092.3 (framework decomposition and documentation), T-1092.4 (framework pin and CI),
T-1092.5 (tbd-emcp), T-1092.6 (tbd-export). Execution: [plan](/documentation/tickets/plans/t-1092_plan.md);
progress: [checkpoint](/documentation/mod/script_modularisation_progress_checkpoint.md).

## In one sentence

Every Enfusion script in the three mod addons is split by responsibility to at most 500 lines,
documented to the Enfusion comment standard, and held there by two CI gates.

## Problem

- Law 7: 52 `.c` files exceed 500 lines (36 in tbd-framework, 15 in tbd-export, 1 in tbd-emcp);
  `Systems/Spawning/TBD_SpawnManager.c` is 4231. `cargo xtask verify file-length` walks only
  `.rs` files, so nothing reports it.
- Law 8: in tbd-framework, 169 of 172 files lack the `/** */` header, 113 contain non-ASCII,
  about 950 comment lines cite tickets and 650 carry history wording, 2,522 lines are separators,
  42% of methods and 62% of fields are undocumented, `@replicated` covers 1 of 7 `[RplProp]` and
  `@route` none of 12 HTTP call sites. No gate checks any of it.
- The same helpers are written several times over (dead-body checks, chat, wire codecs, JSON
  second pass, eight near-identical tick drivers).

## Goal

- Every `.c` file under `apps/mod/{tbd-framework,tbd-emcp,tbd-export}/Scripts` is at or under
  500 lines, one primary type per file, with subfolders where a file became three or more.
- Duplicates are merged into shared helpers; one ordered runtime heartbeat replaces the eight
  `modded SCR_BaseGameMode` tick drivers; same-addon `modded class` blocks are folded into their
  bases.
- Every file meets the Enfusion standard (header, banners, trailing field docs, network and
  cross-boundary tags, present-tense ASCII comments).
- `cargo xtask verify file-length` covers the three addon Scripts roots, and
  `cargo xtask verify enfusion-comments` checks the standard; both run in `ci-local` and `ci.yml`.
- Behaviour is unchanged.

## Out of scope

- New gameplay behaviour, JSON key renames, RPC renames, class renames referenced by prefabs,
  layouts or configs.
- `apps/mod/crf_framework` and `apps/mod/vanilla_reference` (gitignored references).

## Locked decisions

Operator, 2026-09-26:

- The file header is the `/** */` block of `documentation_standards.md` section 6.
- All three addons are gated; tbd-export starts only after the other session commits its
  export work.
- The comment gate carries the full rule set and is wired into CI.
- Consolidation includes the heartbeat merge.
- The vendored `EMCP_WB_ModifyEntity.c` is split; its README records the divergence from
  enfusion-mcp@0.6.1.
- Every separator line is removed and banned.
- tbd-emcp and tbd-export compile checks run in the Workbench Script Editor (operator).

## Tasks

The roster, slices and launch prompts are in the [plan](/documentation/tickets/plans/t-1092_plan.md).

## Verify

- `cargo xtask mod compile` exits 0.
- `cargo xtask mod world-boot` exits 0 and the heartbeat's order line matches the recorded
  baseline.
- `cargo xtask verify enfusion-comments` exits 0.
- `cargo xtask verify file-length` exits 0 and reports the `.c` count.
- `cargo xtask ci ci-local` passes.
- Operator: a two-client playtest behaves as before; tbd-emcp and tbd-export compile clean in
  Workbench.

## Documentation

- Every changed folder's `README.md`; new subfolders get one from
  `documentation_v2/standards/templates/readme_mod_scripts.md`.
- `documentation_v2/standards/templates/enfusion_script_header.md` (new).
- `CLAUDE.md` laws 7 and 8 and section 3; `documentation_standards.md` sections 3.1, 6 and 7;
  `coding_standards/{file_size_and_complexity,enfusion_code_policy,README,ci_gates}.md`.
- `tools_v2/xtask` READMEs for the changed verifications and commands.
- `documentation_v2/mod/` feature docs that cite moved paths.

## Claude Code prompt — T-1092 (copy-paste)

The program runs from the prompts in the [plan](/documentation/tickets/plans/t-1092_plan.md);
resume by reading the [checkpoint](/documentation/mod/script_modularisation_progress_checkpoint.md).
