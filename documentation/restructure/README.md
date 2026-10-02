**Status:** live

# Workspace restructure program

The live record of the program that rebuilds the Rust workspace from the foundations up into
standard, fine-grained crates. It removes the `_v2` names, flattens the apps, replaces the map
engine's feature matrix with real crate boundaries and applies per-crate code standards. Any
session, including a fresh one with no chat history, resumes the program from this folder.

## Contents

```text
documentation/restructure/
├── agent_briefs/         the shared agent brief, and a running stage's agent documents
├── crate_catalogue.md    every crate, built or planned: category, tier, source paths, dependencies, what it fixes
├── laws_and_gates.md     the new repository laws, the standard gate set, gate evolution, end-state checks
├── manifests/            the relocation manifests the stages run, and the format sample
├── program_plan.md       context, verified findings, binding decisions, execution model, stages, agents, risks
├── progress.md           the progress tracker: stage and agent rows, execution log, findings, handoff
├── stage_logs/           the parallel stages' own logs (S3–S6) and the protocol they follow
└── target_file_tree.md   the exact end-state file tree and the crate anatomy
```

## How it works

To resume, read in this order:

1. [progress.md](/documentation/restructure/progress.md): its header and Handoff section name
   the current stage, the last green commit and the exact next step.
2. [program_plan.md](/documentation/restructure/program_plan.md): the decisions, the strangler
   execution model and the stage table with each agent's role, budget and owned files.
3. [target_file_tree.md](/documentation/restructure/target_file_tree.md) and
   [crate_catalogue.md](/documentation/restructure/crate_catalogue.md): the goal.
4. [laws_and_gates.md](/documentation/restructure/laws_and_gates.md): what each stage must pass.

The program runs the [sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md)
method. One [orchestrator](/documentation/glossary/n_to_z.md#orchestrator) session launches
file-disjoint agents wave by wave, gates between waves and commits once per stage. The ticket
registry is not used for this program: `progress.md` is the single tracker. The orchestrator
updates it after every agent report and gate, and commits and pushes it at every wave boundary.

Planned paths that do not exist yet are written as plain text or inside `text` blocks, never as
inline code under an existing top-level folder. Planned `xtask` subcommands are written without
the `cargo` prefix, since the link check requires every cited `cargo xtask` command to exist.
Both conventions keep the documentation gates green while the documents describe the future
tree. The research the plan rests on is archived in
[restructure research](/documentation/archive/restructure_research/README.md).

## Code

- [Map engine](/legacy/map_engine/) and [graphics engine](/legacy/graphics_engine/) —
  the monoliths the program dissolves into tiered crates.
- [API](/apps/api/), [frontend](/apps/frontend/) and
  [tooling](/tools/) — the other monoliths split into kernel, domain, page and tool crates.
- [Mod](/apps/mod/) — the reference-folder consolidation and the objective behaviour classes.

## Boundaries

- Depends on: [CLAUDE.md](/CLAUDE.md) laws; the
  [documentation standards](/documentation/standards/documentation_standards.md); the
  [engine boundary rules](/documentation/standards/engine_boundary_rules.md) the program
  replaces with crate-level laws.
- Used by: the program's orchestrator and agents; `CLAUDE.md`, `AGENTS.md` and the documentation
  entry README, which point here while the program runs.
- Rules: `progress.md` is the only tracker; every stage commit leaves it current; when the program
  closes, the folder moves to the archive and the pointers are removed.

## Related documentation

- [Architecture blueprint](/documentation/archive/restructure_research/00_architecture_blueprint_draft.md) — the operator's draft
  this program corrects and supersedes.
- [Restructure research](/documentation/archive/restructure_research/README.md) — the explorer,
  planning and verification reports behind every finding.
- [Archived agent briefs](/documentation/archive/restructure_agent_briefs/README.md) — the briefs
  the finished stages' agents ran.
- [Workspace layout](/documentation/architecture/workspace_layout.md) — the workspace as it stands
  after the latest stage.
