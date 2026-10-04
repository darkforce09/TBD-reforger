**Status:** live

# BTR-70 render

Design-phase reference for the image area of the vehicle database dossier at `/vehicles`: a
top-down render of a BTR-70 on a dirt track among supply crates and camouflage nets, framed by
"MILITARY INTELLIGENCE" and "MILITARY INTELLIGENCE ANALYSIS" labels. It gives mood and colour
context and is not an implementation source; the built UI is the Leptos code under
`crates/frontend/pages/doctrine_pages/src/vehicles/`.

## Contents

```text
documentation/crates/frontend/pages/doctrine_pages/vehicles/visual_references/btr_70_render/
└── btr_70_render.png  the render
```

## How it works

The render is a single image with no Stitch export. The built dossier differs: its image area
shows the vehicle row's own profile image, or a vehicle icon when the row has none, with no
intelligence framing. The
[vehicle database page](/documentation/crates/frontend/pages/doctrine_pages/vehicles/vehicle_database_page.md)
feature doc holds the full comparison.

## Code

- [Vehicle database page](/crates/frontend/pages/doctrine_pages/src/vehicles/) — the
  page this render was made for.

## Boundaries

- Depends on: nothing; the png is self-contained.
- Used by: the vehicle database feature doc's Design section and the visual references README.
- Rules: the render is kept as captured and named after the set.
