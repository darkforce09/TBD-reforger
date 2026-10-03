**Status:** live

# API completion records

The records of the program that completes the website [API](/documentation/glossary/a_to_f.md#api)
milestone by milestone against its acceptance register: the execution records of finished
milestones, moved out of the live verification checkpoint once a later milestone closed.
Status: archived — frozen records.

## Contents

```text
documentation/archive/api_v2_completion/
├── milestone_b_execution_record.md  milestone B's phase handoffs and the additions to its agent prompts
├── milestone_c_execution_record.md  milestone C's phase handoffs and the additions to its agent prompts
└── milestone_v_execution_record.md  milestone V's phase handoffs, the additions to its agent prompts and its findings pointer
```

## How it works

Each file holds one milestone's execution record exactly as the checkpoint carried it: a dated
row per phase handoff, then the additions made to the pre-written agent prompts at launch. Read
it for how a milestone was run; the live checkpoint and the design notes say what the API does
now. Paths, counts and log names in the records are as they stood when each row was written.

## Code

- [API crate](/apps/api/) — the crate the milestones completed.
- [API readiness check](/tools/commands/api_readiness_checks/src/) — the verifier that
  judges the register the milestones fill.

## Boundaries

- Depends on: nothing live; the records quote the code and the logs of their time.
- Used by: the verification checkpoint, which points here for the milestone B, C and V records.
- Rules: never reworded, only links change; a milestone's record moves here verbatim when the
  live checkpoint needs its room.

## Related documentation

- [Verification checkpoint](/documentation/apps/api/verification_evidence/progress_checkpoint.md)
  — the live state of the program and the records of the milestones still in it.
- [API verification evidence](/documentation/apps/api/verification_evidence/README.md) —
  the register, the design notes and the program records.
