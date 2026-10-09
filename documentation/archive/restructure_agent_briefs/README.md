**Status:** live

# Restructure agent briefs

The agent briefs of the workspace restructure program's finished stages: the documents each
stage's implementing agents were launched with, kept as the record of what every agent was asked
to do. Status: archived — frozen records; they quote the paths, names and line numbers of the tree
their stage started from.

## Contents

```text
documentation/archive/restructure_agent_briefs/
├── s1_r1_global_renames.md    S1 agent R1: the scripted global renames with the relocation tool
├── s1_r2_to_r4_template.md    S1 agents R2 to R4: the area fix-up template filled from R1's report
├── s2_a1_apps_and_deploy.md   S2 agent A1: the website crates to apps/ and legacy/, snake_case packages, deploy/
└── s2_a2_to_a6.md             S2 agents A2 to A6: deploy, wave execution, runtime paths, crate births, documentation, and the operator steps
```

## How it works

A stage's briefs move here at the stage's commit, named `s<stage>_<agent ids>_<topic>.md`; the
program's own `agent_briefs/` folder holds only the shared brief every agent read first. A brief whose relocation manifest
was drafted inside it carries a pointer to the committed manifest in place of the draft. The
operator steps a stage leaves behind stay in its brief: the runbooks link the
[S2 operator steps](/documentation/archive/restructure_agent_briefs/s2_a2_to_a6.md#oc-deploy-operator-steps),
the one-time host moves of the deploy folder and the snake_case names.

## Code

- [API](/apps/api/), [frontend](/apps/frontend/), [deploy](/deploy/),
  [library crates](/crates/), [legacy engines](https://github.com/darkforce09/TBD-reforger/tree/2a105fa4fbc0a23062fcff3dc5aedd387127c5d9/legacy) and [tooling](/tools/) — the code the
  briefs moved and changed.

## Boundaries

- Depends on: the [shared agent brief](/documentation/archive/restructure/agent_briefs/shared_brief.md),
  which every brief here names as its first read.
- Used by: the [restructure program](/documentation/archive/restructure/README.md), whose progress log
  cites the briefs; the website, staging and CI runbooks, which link the S2 operator steps.
- Rules: never reworded, only links change; each brief carries `**Status:** archived`; a stage's
  briefs land here at its commit.

## Related documentation

- [Restructure program](/documentation/archive/restructure/README.md) — the plan, target tree and progress
  the briefs executed.
- [Relocation manifests](/documentation/relocation_manifests/README.md) — the committed manifests
  the briefs ran.
- [Restructure research](/documentation/archive/restructure_research/README.md) — the reports the
  plan rests on.
