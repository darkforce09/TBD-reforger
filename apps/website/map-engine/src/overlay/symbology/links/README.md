# Squad links

The leader-to-member tether hairlines of each squad on the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map, tinted by side, at rest and
during a drag.

## Contents

```text
apps/website/map-engine/src/overlay/symbology/links/
├── mod.rs          the module tree
├── squad_links.rs  `SquadLinkInput` and the squad tether vertex packers, at rest and mid-drag
└── tests/          unit tests for segment counts, side colour, missing positions and drag previews
```

## How it works

`build_squad_link_segments` emits one segment from the leader to each other member of every
`SquadLinkInput`, as a LineList of six floats per vertex (`[x, y, r, g, b, a]`), coloured by
`crate::overlay::symbology::roles::classify::side_rgba` of the squad's side. A squad with no
leader, or whose member list does not include its leader, draws nothing, and a member with no
position is skipped. `pack_squad_link_drag_preview` offsets the dragged
[slot](/documentation_v2/glossary.md#slot) ids by `(dx, dy)` and
repacks only the squads a dragged id belongs to; an empty drag or a zero delta returns the rest
layout.

## Boundaries

- Depends on: `crate::overlay::symbology::roles::classify::side_rgba`.
- Used by: `crate::editing::picking::squad_link_inputs`, which builds the inputs from the document;
  the Mission Creator's document host
  (`apps/website/frontend/src/v2/apps/editor/bridge/document_host/history.rs`) for the rest layout
  and its select tool (`apps/website/frontend/src/v2/apps/editor/input/tools/select_tool.rs`) for
  the drag preview.
- Rules: only squads touched by a drag re-resolve with the offset
  (`squad_link_drag_preview_repacks_only_affected_squads` in `tests/squad_links_tests.rs`); a
  squad of one draws no segment (`squad_link_solo_zero_segments`).
