**Status:** live

# API restructuring records

The records of the program that reorganised the website [API](/documentation_v2/glossary.md#api)
crate into `core/`, the background workers and eight domain folders: its inventory of the old
layout, its target architecture and one handoff per phase. Status: archived — frozen records.

## Contents

```text
documentation_v2/archive/api_v2_refactor/
├── analysis_and_inventory.md  the old backend layout, file by file, with the target of each file
├── architecture_plan.md       the target domain architecture, router layout and phased plan
└── phase_*_handoff.md         one handoff per phase, one to seven: what landed and what came next
```

## How it works

Read the architecture plan first, then the handoffs in phase order; each handoff lists the commits
of its phase and the state it left. The files are frozen: paths, commands and counts in them are
as they stood when each was written. The live description of the crate is its code README and the
API documentation, which the Related documentation below links.

## Code

- [API crate](/apps/website/api_v2/) — the crate the program restructured.
- [API source](/apps/website/api_v2/src/) — the `core/`, `background_workers/` and domain folders
  the plan describes.

## Boundaries

- Depends on: nothing live; the records quote the code of their time.
- Used by: the documentation program's own records at the documentation root (the move manifest,
  the pin catalogue and the program plan) and nothing else.
- Rules: never reworded, only links change; no file is added once the program is closed.

## Related documentation

- [API documentation](/documentation_v2/website/api_v2/README.md) — the live overview, environment
  reference, decisions and verification evidence.
- [API crate README](/apps/website/api_v2/README.md) — the crate as it is.
