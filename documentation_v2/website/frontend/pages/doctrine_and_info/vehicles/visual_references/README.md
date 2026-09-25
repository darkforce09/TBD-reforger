**Status:** live

# Vehicle database design references

The design references of the `/vehicles` page: two design-phase sets, a dossier blueprint and a
render, kept for colour and layout context rather than as implementation sources.

## Contents

```text
documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/visual_references/
├── btr_70_render/              a top-down render of a BTR-70 under intelligence-report framing
└── vehicle_dossier_blueprint/  a faction list beside a BTR-70 identification dossier
```

## How it works

A set is a folder named after its subject and kind. A blueprint holds the Stitch export as an html
file and its screenshot as a png; a render holds only its png; each is named after the set and has
a README that says what it shows and how the built page differs. The built UI is the Leptos code
the Code section links; the
[vehicle database page](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md)
feature doc lists every difference in its Design section.

## Code

- [Vehicle database page](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/) — the
  built page the sets were drawn for.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from the styles, fonts
  and images a blueprint's html loads from the network.
- Used by: the page's feature doc, whose Design section links both sets, and the page folder
  README.
- Rules: a set is kept as captured and never edited to match the built page; a new set gets its
  own folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a
  README; no screenshot of the built UI belongs here.
