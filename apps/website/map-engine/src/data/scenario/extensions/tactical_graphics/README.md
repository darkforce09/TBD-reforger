# Tactical graphics

The check on a [mission](/documentation_v2/glossary.md#mission)'s authored `tacticalGraphics`
block: the control measures a one-point map marker cannot draw (phase lines, boundaries, axes of
advance and curved arrows), each an ordered run of world vertices with an optional label, side and
stroke style. The module is exposed as `data::scenario::tactical_graphics`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/tactical_graphics/
├── mod.rs     the module tree; re-exports the graphic types, the vocabularies, limits and checks
├── quote.rs   the value quoting and JSON type names the refusal sentences use
├── shapes.rs  `parse` and `validate`; `KINDS`, `BRUSHES`, `MAX_POINTS`, `min_points`, the style
└── tests/     unit tests for each kind, each refusal and the block's way onto the payload root
```

## How it works

`parse` reads a non-empty array of graphics as `contracts_v2/definitions/mission.schema.json`
shapes each one in `$defs/tacticalGraphic`: `{id, kind, points, label?, sideKey?, style?}`. `kind`
is one of `KINDS` (`phase_line`, `boundary`, `axis_of_advance`, `curved_arrow`). `points` holds
`[x, z]` pairs of finite numbers, from `min_points(kind)` (2, or 3 for a `curved_arrow`, a floor
per kind that the schema leaves to this check) up to `MAX_POINTS` (128). `sideKey` has the
faction-key form `^[a-z][a-z0-9_]*$` and is not checked against the mission's sides. `style` is
`{color?, alpha?, brush?, widthM?}`: a `#rrggbb` colour, an alpha from 0 to 1, one of `BRUSHES`
(the eight brush names map markers use) and a stroke width above zero in world metres. A `null`
label, side or style counts as absent, ids are unique, and a key the schema does not declare
refuses the block. The first problem found is the answer, a sentence with its path, such as
"`tacticalGraphics[0]`.points has 1 point(s) — a phase_line needs at least 2".

`validate` is `parse` with the value dropped: the check of the `tacticalGraphics` row of
`AUTHORED_BLOCKS` in `crate::data::scenario::extensions`. The compile carries a valid block
verbatim to the compiled document's root, where it stops: no script of the
[mod](/documentation_v2/glossary.md#mod) reads it. The
[Mission Creator](/documentation_v2/glossary.md#mission-creator) draws the graphics on its map,
and its draw tool takes the same floor and cap, so a drawn graphic always has a vertex count this
check accepts.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `tacticalGraphics` row calls `validate`;
  `crate::data::store::operations`
  (`apps/website/map-engine/src/data/store/operations/tactical_graphics.rs`), the Mission Creator's
  draw tool, which arms only a kind `min_points` knows, completes a draft at its floor and stops
  adding vertices at `MAX_POINTS`; the Mission Creator's zones panel
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/zone_list_panel.rs`), which
  offers one draw button per entry of `KINDS`.
- Rules: each kind's vertex floor holds (`a_one_point_phase_line_is_refused` and
  `a_two_point_curved_arrow_is_refused_but_a_two_point_axis_is_not` in `tests/cases_1.rs`);
  `KINDS` and `MAX_POINTS` are pinned (`tactical_graphics_is_registered_on_the_carrier`), and the
  schema's `maxItems` of 128 mirrors `MAX_POINTS`; a style outside the marker vocabulary is refused
  (`a_style_outside_the_marker_vocabulary_is_refused`); a valid block passes the carrier whole
  (`tactical_graphics_registered_here_must_also_be_readable_by_flatten`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `tacticalGraphics`,
  `$defs/tacticalGraphic` and `$defs/tacticalGraphicStyle`, the shape this module checks.
