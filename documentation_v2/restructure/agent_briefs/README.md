**Status:** live

# Restructure agent briefs

The briefs the restructure program's agents are launched with: the shared brief every agent reads
first, and one document per agent (or agent template) of the stage that comes next. They live in
the repository so a session on any machine can continue the program.

## Contents

```text
documentation_v2/restructure/agent_briefs/
├── s1_r1_global_renames.md     S1 agent R1: the scripted global renames, with the manifest draft
├── s1_r2_to_r4_template.md     S1 agents R2 to R4: the area fix-up template filled from R1's report
└── shared_brief.md             rules, efficiency, spec and report format every agent follows
```

## How it works

The orchestrator launches an agent with a one-line prompt that points at its document here, names
a scratch folder outside the repository for logs and probes, and adds any machine-specific
settings (an environment file). A stage's briefs are written before it starts and move to the
archive with the program at its close.

## Boundaries

- Depends on: the [program plan](/documentation_v2/restructure/program_plan.md) and the
  [sub-agent orchestration](/documentation_v2/runbooks/sub_agent_orchestration.md) runbook.
- Used by: the orchestrator session and the agents it launches.
- Rules: an agent document names its owned files and checks; it never asks an agent to commit.

## Related documentation

- [Progress](/documentation_v2/restructure/progress.md) — the current stage and the next step.
