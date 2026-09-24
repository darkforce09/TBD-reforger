# ORBAT slot lines

One plain-text line per [slot](/documentation_v2/glossary.md#slot) of an
[ORBAT](/documentation_v2/glossary.md#orbat): its 1-based number, role, weapons, tag and leader
mark, as the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s ORBAT manager
lists it.

## Contents

```text
apps/website/map-engine/src/data/scenario/slot_line/
├── format_slot_line.rs  `format_slot_line` and the weapon resolution it reads
├── mod.rs               the module tree; re-exports `format_slot_line`
└── tests/               unit tests for each part of the line
```

## How it works

`format_slot_line(index_1based, role, summary, primary, launcher, tag, is_leader)` writes
`2: Medic (L85A3) | MED` and, with every part, `1: Squad Leader (L85A3 + GL) | MED | SL`. The
weapons come from `primary` and `launcher` when either is set, otherwise from `summary` split at
` · ` into primary and launcher; blank parts are left out, and ` | SL` follows only when
`is_leader` is true.

## Boundaries

- Depends on: nothing outside the standard library.
- Used by: the Mission Creator's ORBAT manager tree
  (`apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager/tree_rows.rs`), which marks
  the leader with an icon and passes `is_leader` false.
- Rules: the line's format is pinned part by part (`format_slot_line_primary_and_launcher`,
  `format_slot_line_summary_dot_split`, `format_slot_line_is_leader` in `tests/cases_1.rs`).
