**Status:** live

# Mortar calculator page documentation

The feature documentation of the `/tools/mortar` page, where anyone solves a battery's mortar
firing solutions on the device from a ballistics catalog, online or offline, and a signed-in member
saves them to an [event](/documentation/glossary/a_to_f.md#event), with the page's design-phase
reference.

## Contents

```text
documentation/crates/frontend/pages/field_tools_pages/mortar/
├── mortar_calculator_page.md  the feature doc: catalogs, placing, solving, offline use, saving, the API
└── visual_references/         the design-phase blueprint of a map with draggable markers and a HUD
```

## How it works

Read [mortar_calculator_page.md](/documentation/crates/frontend/pages/field_tools_pages/mortar/mortar_calculator_page.md)
first. It follows the [feature doc template](/documentation/standards/templates/feature_doc.md):
it quotes the page's interface text, walks through choosing a catalog, placing the battery and the
target, solving, working offline and saving, gives what each catalog and fire-mission route of the
[API](/documentation/glossary/a_to_f.md#api) does, and compares the built page with the blueprint
in `visual_references/`. The blueprint is a design-phase reference: the built page keeps its map
with draggable markers and replaces its three readouts with a whole solution panel. The model the
page solves with is documented in the
[game ballistics engine](/documentation/crates/ballistics/game_ballistics_engine.md)
feature doc; the code folder's README lists the page's files.

## Code

- [Mortar calculator page](/crates/frontend/pages/field_tools_pages/src/mortar/) — the route
  component `MortarCalculatorPage`, the inputs, the map picker, the solve bridge, the solution
  panel, the save area and the offline pack line.
- [Game ballistics](/crates/ballistics/README.md) — the
  `solve_fire_mission` assembler the page and the API run.
- [Offline core](/crates/frontend/foundation/frontend_offline/src/) — the offline pack the page
  triggers.
- [Operations domain](/crates/api/api_operations/src/) — the ballistics catalog and
  fire-mission routes.

## Boundaries

- Depends on: the feature doc template; the page code, the catalog and fire-mission handlers, the
  ballistics crates, the offline core and the ticket registry the feature doc is written
  from.
- Used by: the in-code READMEs of the mortar page and of the field tools pages, which link the
  feature doc; the field tools pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Operations domain](/crates/api/api_operations/src/README.md) — the API side of the
  catalogs and the fire missions.
- [Game ballistics design note](/documentation/apps/api/verification_evidence/game_ballistics.md)
  — the operator decisions, the model, the tolerances and the register.
- [Offline mortar page](/documentation/runbooks/offline_mortar_page.md) — preparing, using and
  checking the offline pack.
