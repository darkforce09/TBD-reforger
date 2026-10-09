**Status:** live

# Frontend reorganisation log

The log of the program that moved the frontend's code, folder by folder, into the domain tree
under `apps/website/frontend/src/v2/`: the rules every phase followed and what each phase moved.
Status: archived — frozen records.

## Contents

```text
documentation/archive/frontend_v2_migration/
└── migration.md  the phase rules and the per-phase log of the move into src/v2
```

## Code

- [Frontend domain tree](/crates/frontend/shell/frontend_application/src/) — the tree the move produced.

## Boundaries

- Depends on: nothing live; the log quotes the tree of its time.
- Used by: the documentation program's own records at the documentation root and nothing else.
- Rules: never reworded, only links change.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) — the frontend as it is,
  with every route and its page folder.
- [Frontend domain tree README](/crates/frontend/shell/frontend_application/src/README.md) — the code layout.
