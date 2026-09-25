**Status:** live

# Mortar calculator design references

The design references of the `/tools/mortar` page: one design-phase blueprint set, kept for colour
and layout context rather than as an implementation source.

## Contents

```text
documentation_v2/website/frontend/pages/field_tools/mortar/visual_references/
└── mortar_calculator_blueprint/  a gridded map with draggable markers and a firing-solution HUD
```

## How it works

A set is a folder named after its subject and kind. It holds the Stitch export as an html file
and its screenshot as a png, both named after the set, and a README that says what the set shows
and how the built page differs. The built UI is the Leptos code the Code section links; the
[mortar calculator page](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
feature doc lists every difference in its Design section.

## Code

- [Mortar calculator page](/apps/website/frontend/src/v2/pages/field_tools/mortar/) — the built
  page the set was drawn for.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from the styles and
  fonts its html loads from the network.
- Used by: the page's feature doc, whose Design section links the set, and the page folder README.
- Rules: a set is kept as captured and never edited to match the built page; a new set gets its
  own folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a
  README; no screenshot of the built UI belongs here.
