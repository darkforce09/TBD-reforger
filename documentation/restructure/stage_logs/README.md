**Status:** live

# Restructure stage logs

Every remaining stage (S3–S12 and M3) runs in its own detached git worktree under its own
orchestrator session (decisions D21 and D22), managed by one coordinator session. This folder holds one log per stage, so the four sessions never
append to the same file, together with the protocol every one of them follows.

## Contents

```text
documentation/restructure/stage_logs/
├── m3.md    M3 objective behaviours: execution log, amendments, findings
├── s10.md   S10 frontend crates: execution log, amendments, findings
├── s11.md   S11 tools: execution log, amendments, findings
├── s12.md   S12 close: execution log, amendments, findings
├── s3.md    S3 frontend in place: execution log, amendments, findings
├── s4.md    S4 tier 0–1 foundations: execution log, amendments, findings
├── s5.md    S5 mission and ballistics: execution log, amendments, findings
├── s6.md    S6 world CPU: execution log, amendments, findings
├── s7.md    S7 streaming CPU and editor: execution log, amendments, findings
├── s8.md    S8 rendering: execution log, amendments, findings
└── s9.md    S9 API: execution log, amendments, findings
```

## How it works

### Worktrees

Each stage runs in a detached worktree beside the main checkout, so no branch exists (CLAUDE.md
law 2): `/run/media/system/Disk_2/Projects/tbd-restructure-<stage>`. A worktree is created from
`main` with Git LFS smudging skipped. A test that needs LFS data pulls only its paths
(`git lfs pull --include <path>`). The gitignored reference lanes stay in the main checkout only (a
symbolic link to them breaks `git check-ignore`), and the API's development environment file is a
private copy. Each worktree's run folder,
`target/restructure-<stage>/`, holds its `env.sh`, logs, probes, agent prompts and the
orchestrator's launch prompt.

### Coordinator

One session, "Restructure coordinator", manages every stage orchestrator with the operator's
delegation (decision D22). An orchestrator sends it its question rounds (options with a recommended
one and evidence), its plan, its status at wave boundaries, its blockers and "landed <sha>". Each
stage plans first (`target/restructure-<stage>/PLAN.md`, its brief and agent prompts, no repository
edits) and executes only after the coordinator's "plan approved". The coordinator escalates to the
operator anything that changes a program decision, touches deployments or servers, or deletes data.
A stage whose inputs are not on `main` waits for the coordinator's "go":

| Stage | Inputs on `main` before it executes |
|---|---|
| S3 | none |
| S4 | none |
| S5, S6 | S4's tier 0–1 crates |
| S7 | S5, S6 |
| S8 | S6, S7 |
| S9 | S5 (plans before) |
| S10 | S3, S7, S8 |
| S11 | S4 (J0–J2), S6 (J3), S8 (the empty legacy folder) |
| M3 | none |
| S12 | every other stage |

### Records

- A stage writes its execution log, amendments and findings in its own file here. Finding ids carry
  the stage: `F-S3-01`, `F-S4-01`, and so on.
- In [progress](/documentation/restructure/progress.md) a stage ticks only its own checklist rows.
  The "Current state" table and the Handoff change only in a stage's final rebased commit.

### Shared files

These files are edited own-lines-only, with a re-read right before each edit:

- the root `Cargo.toml` (new crates are members through the `crates/*/*` glob);
- `CLAUDE.md` §2 (the stage's own subtree lines only);
- `documentation/restructure/crate_catalogue.md` ("Built so far" rows are added in the final rebased commit);
- the `repository_layout` crate's `src/lib.rs` and location modules (`tools/foundation/repository_layout/src/`);
- `legacy/map_engine/src/lib.rs` and the map engine's features;
- the manifests README (the stage's own row).

Manifests are named after their stage (`s3_*.tsv`, `s4_*.tsv`, and so on) and never change once
on `main`. On a `Cargo.lock` rebase conflict, take `main`'s version, regenerate it with
`cargo metadata --format-version 1`, and check that only the stage's own packages changed.

### Dependencies and order

- S5 and S6 create crates only after S4 is on `main`, because no new crate may depend on `legacy/`.
  S5 needs `newtype_ids`, `time_source`, `deterministic_random`, `content_digest` and
  `map_coordinates`. S6 needs `geometry_primitives`, `map_coordinates`, `world_file_formats` and
  `render_primitives`.
- Until S4 lands, S5 and S6 do their research, briefs and cut waves only. Before a birth wave they
  check the main checkout's history for the S4 stage commit. If it is absent, they stop with
  "blocked on S4 (pre-work done)", and the operator resumes them once S4 lands.
- S3 is independent apart from the shared files.

### Landing a stage

1. The stage gate is green (decision D20).
2. Rebase the stage's commits onto the current `main` in the worktree, resolve conflicts, and
   re-run the stage gate.
3. Fast-forward `main` from the worktree:
   `git -C /run/media/system/Disk_2/Projects/TBD-Reforger merge --ff-only <sha>`. If `main` has
   moved, go back to step 2.
4. Push from the container session (git-lfs): `git -C <main checkout> push origin main`.

A stage lands as one commit, `refactor(restructure): <stage> <title>`, preceded by any number of
`docs(restructure): progress <stage>/<wave>` commits.

## Boundaries

- Depends on: the [program plan](/documentation/restructure/program_plan.md) (decisions D20, D21)
  and the [sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md) runbook.
- Used by: the four stage orchestrators and the agents they launch.
- Rules: a stage writes only its own log; findings keep their stage prefix after the program closes.

## Related documentation

- [Progress](/documentation/restructure/progress.md) — the stage checklists and the handoff.
