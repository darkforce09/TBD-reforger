**Status:** live

# Mortar calculator page documentation

The feature documentation of the `/tools/mortar` page, where members compute a mortar firing
solution and save it to an [event](/documentation_v2/glossary/a_to_f.md#event), with the page's
design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/field_tools/mortar/
├── mortar_calculator_page.md  the feature doc: solving, saving to an event, the solver and the API
└── visual_references/         the design-phase blueprint of a map with draggable markers and a HUD
```

## How it works

Read [mortar_calculator_page.md](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives the solver's model and what each fire-mission route of
the [API](/documentation_v2/glossary/a_to_f.md#api) does, and compares the built page with the blueprint
in `visual_references/`. The blueprint is a design-phase reference: it drags markers on a map,
while the built page takes four typed coordinates and adds the tube and event pickers, the charge,
the time of flight and the saved list. The code folder's README lists the page's files.

## Code

- [Mortar calculator page](/apps/website/frontend/src/v2/pages/field_tools/mortar/) — the route
  component `MortarCalculatorPage`, the inputs, the preview, the solution card and the saved list.
- [Operations domain](/apps/website/api_v2/src/operations/) — the fire-mission routes.
- [Mission data ballistics](/apps/website/map-engine/src/data/scenario/ballistics/) — the solver
  the API calls.

## Boundaries

- Depends on: the feature doc template; the page code, the fire-mission handlers, the solver and
  the ticket registry the feature doc is written from.
- Used by: the in-code READMEs of the mortar page and of the field tools pages, which link the
  feature doc; the field tools pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the API side of the fire
  missions.
