**Status:** live

# Mortar calculator blueprint

Design-phase reference for the mortar calculator page at `/tools/mortar`: a gridded tactical map
with a firing position and a target joined by a line, and a heads-up panel with the firing
solution. It gives colour and layout context and is not an implementation source; the built UI is
the Leptos code under `apps/website/frontend/src/v2/pages/field_tools/mortar/`.

## Contents

```text
documentation_v2/website/frontend/pages/field_tools/mortar/visual_references/mortar_calculator_blueprint/
├── mortar_calculator_blueprint.html  the Stitch export of the calculator
└── mortar_calculator_blueprint.png   its screenshot
```

## How it works

The blueprint shows a map on a fine grid with two markers and the line between them, readouts of
"Distance" (1,240 m), "Azimuth" (342.5°) and "Elevation" (1084 mils), a "Firing Solution" panel
with the same three figures, and the hint "Drag markers on the map to recalculate.".

The built page differs: the positions are typed into four number fields, and the preview draws
the points on a plain frame with nothing to drag; the solution adds the charge and the time of
flight; and the tube and [event](/documentation_v2/glossary/a_to_f.md#event) pickers, the saved state
line and the saved fire missions are additions. The
[mortar calculator page](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
feature doc holds the full comparison.

## Code

- [Mortar calculator page](/apps/website/frontend/src/v2/pages/field_tools/mortar/) — the page
  this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the mortar calculator feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
