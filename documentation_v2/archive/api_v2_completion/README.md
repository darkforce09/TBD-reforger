**Status:** live

# API completion records

The records of the program that completes the website [API](/documentation_v2/glossary/a_to_f.md#api)
milestone by milestone against its acceptance register: the execution records of finished
milestones, moved out of the live verification checkpoint once a later milestone closed.
Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/api_v2_completion/
└── milestone_c_execution_record.md  milestone C's phase handoffs and the additions to its agent prompts
```

## How it works

Each file holds one milestone's execution record exactly as the checkpoint carried it: a dated
row per phase handoff, then the additions made to the pre-written agent prompts at launch. Read
it for how a milestone was run; the live checkpoint and the design notes say what the API does
now. Paths, counts and log names in the records are as they stood when each row was written.

## Code

- [API crate](/apps/website/api_v2/) — the crate the milestones completed.
- [API readiness check](/tools_v2/xtask/src/verifications/api_readiness/) — the verifier that
  judges the register the milestones fill.

## Boundaries

- Depends on: nothing live; the records quote the code and the logs of their time.
- Used by: the verification checkpoint, which points here for the milestone C record.
- Rules: never reworded, only links change; a milestone's record moves here verbatim when the
  live checkpoint needs its room.

## Related documentation

- [Verification checkpoint](/documentation_v2/website/api_v2/verification_evidence/progress_checkpoint.md)
  — the live state of the program and the records of the milestones still in it.
- [API verification evidence](/documentation_v2/website/api_v2/verification_evidence/README.md) —
  the register, the design notes and the program records.
