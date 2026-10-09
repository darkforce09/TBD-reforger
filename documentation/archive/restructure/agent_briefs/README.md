**Status:** archived

# Restructure agent briefs

The brief every agent of the restructure program reads first, and, while a stage runs, the
documents its agents are launched with. They live in the repository so a session on any machine
can continue the program.

## Contents

```text
documentation/archive/restructure/agent_briefs/
└── shared_brief.md  rules, efficiency, spec and report format every agent follows
```

## How it works

The orchestrator launches an agent with a one-line prompt that points at its document, names a
scratch folder outside the repository for logs and probes, and adds any machine-specific settings
(an environment file). A stage's own briefs are written before it starts; executed briefs move to
the [archive](/documentation/archive/restructure_agent_briefs/README.md) with the relocation tool
at their stage's commit, so this folder holds only the shared brief between stages.

## Boundaries

- Depends on: the [program plan](/documentation/archive/restructure/program_plan.md) and the
  [sub-agent orchestration](/documentation/runbooks/sub_agent_orchestration.md) runbook.
- Used by: the orchestrator session and the agents it launches; every archived brief names the
  shared brief as its first read.
- Rules: an agent document names its owned files and checks; it never asks an agent to commit; an
  executed brief leaves this folder at its stage's commit.

## Related documentation

- [Progress](/documentation/archive/restructure/progress.md) — the current stage and the next step.
- [Archived agent briefs](/documentation/archive/restructure_agent_briefs/README.md) — the briefs
  of the finished stages.
