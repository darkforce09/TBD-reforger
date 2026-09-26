**Status:** live

# Factory run records

The dated records of the agent factory's runs: the procedures, briefs and kickoffs written for one
run or one agent, and the ledgers and checklists a run kept. The factory's lasting procedure and
rules live in the factory waves runbooks. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/factory_runs/
├── editor_factory_for_cursor_2026_08.md  Mission Creator factory handed to a Cursor command center
├── editor_factory_start_2026_08.md       cold-start file of the Mission Creator factory
├── editor_slice_brief_2026_08.md         slice-agent brief of the Mission Creator factory
├── editor_verify_brief_2026_08.md        adversarial verifier brief of the Mission Creator factory
├── eye_pass_2026_09.md                   operator's eye-pass checklist of one factory run
├── factory_for_cursor_2026_07.md         platform factory procedure for a Cursor command center
├── factory_run_2026_09.md                ledger of one run: map storage, audit fixes, idea backlog
├── grok_wave_130_handoff.md              kickoff of one Mission Creator remediation wave range
├── platform_factory_2026_08.md           platform factory procedure: command center, worktrees, waves
└── wave_209_grok_kickoff.md              kickoff of one Mission Creator verifier-findings wave
```

## How it works

A file here is named after its subject and the year and month it was written for; each is a snapshot
of the factory at that time, written for the agent that ran it. The two platform factory procedures
and the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) factory's start file,
briefs and handoff carry a status line pointing at the factory waves runbooks, which hold the
lasting procedure and rules. The ledger, the eye-pass checklist and the two
[wave](/documentation_v2/glossary/n_to_z.md#wave) kickoffs have no live replacement: the landed commits and
the [ticket](/documentation_v2/glossary/n_to_z.md#ticket) files record what the runs shipped.

## Code

- [Platform commands](/tools_v2/xtask/src/commands/platform/) — `cargo xtask platform wave` and the
  slice worktrees the runs drove.
- [Wave commands](/tools_v2/xtask/src/commands/wave/) — the wave lock the runs planned against.

## Boundaries

- Depends on: nothing live; the records quote the tooling of their time.
- Used by: the [factory waves runbook](/documentation_v2/runbooks/factory_waves/README.md), which
  links the platform factory record; `ARCHIVED_WAVE_PLAN_READERS` in
  `tools_v2/ticket-engine/src/repository.rs`, which lets the two wave kickoffs name the archived
  wave plans.
- Rules: never reworded, only links change; a rule still in force lives in the factory waves
  runbooks, not here.

## Related documentation

- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the live factory procedure
  and rules.
- [Mod slice workflow](/documentation_v2/runbooks/mod_slice_workflow.md) — the same process for the
  mod.
