**Status:** live

# Workspace restructure program

The records of the closed program that rebuilt the Rust workspace from the foundations up into
standard, fine-grained crates: it removed the `_v2` names, flattened the apps, replaced the map
engine's feature matrix with real crate boundaries and applied per-crate code standards. It ran in
stages S0 to S12 and M1 to M3, which landed on `main` as `refactor(restructure): <stage> <title>`
commits. Status: archived — frozen records; they quote the paths, counts and plans of their time.

## Contents

```text
documentation/archive/restructure/
├── agent_briefs/         the shared brief every agent of the program read first
├── crate_catalogue.md    every crate the program built or planned: category, tier, source paths, dependencies
├── laws_and_gates.md     the repository laws the program introduced, its stage gate and full gate set
├── program_plan.md       context, verified findings, binding decisions, execution model, stages, agents, risks
├── progress.md           the program's tracker: stage and agent rows, execution log, findings, handoff
├── stage_logs/           each stage's execution log, amendments and findings, and the stage protocol
└── target_file_tree.md   the end-state file tree and the crate anatomy the program aimed at
```

## How it works

The plan states the decisions and the stage table; the progress tracker and the stage logs record
what each stage did, which agent did it and what each finding became. The program's results live
in live documents, which are the authority now:

- the [workspace layout](/documentation/architecture/workspace_layout.md): every workspace member
  and where code, contracts, assets and documents live, in place of the target tree and the crate
  catalogue;
- the [crate boundary rules](/documentation/standards/crate_boundary_rules.md): the crate-tier law,
  the category matrix, the firewalls and every other workspace law as the code enforces it, in
  place of the laws and gates;
- the [relocation manifests](/documentation/relocation_manifests/README.md): every manifest the
  program ran, kept as the retired-spelling registry `cargo xtask refactor relocate --verify`
  reads;
- [CLAUDE.md](/CLAUDE.md): the project laws and the directory atlas.

The research the plan rests on and the briefs of the finished stages' agents are archived beside
this folder, in [restructure research](/documentation/archive/restructure_research/README.md) and
[restructure agent briefs](/documentation/archive/restructure_agent_briefs/README.md).

## Code

- [Apps](/mod/README.md), [crates](/crates/README.md) and [tools](/tools/README.md) — the
  workspace the program restructured.

## Boundaries

- Depends on: nothing live; the records quote the tree of their date.
- Used by: the [archive index](/documentation/archive/README.md); the ticket files and archived
  records that cite the program's decisions and findings.
- Rules: the records are never reworded, only their links change; this README stays a live index
  of them.

## Related documentation

- [Restructure research](/documentation/archive/restructure_research/README.md) — the explorer,
  planning and verification reports behind every finding.
- [Restructure agent briefs](/documentation/archive/restructure_agent_briefs/README.md) — the
  briefs the finished stages' agents ran.
- [Sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md) — the method the
  program's orchestrators followed.
