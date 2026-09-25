**Status:** live

# Modpacks design references

The design references of the `/modpacks` page: one design-phase blueprint set, kept for colour and
layout context rather than as an implementation source.

## Contents

```text
documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/visual_references/
└── modpacks_blueprint/  one wide modpack card with its key dependencies and a connect button
```

## How it works

A set is a folder named after its subject and kind. It holds the Stitch export as an html file
and its screenshot as a png, both named after the set, and a README that says what the set shows
and how the built page differs. The built UI is the Leptos code the Code section links; the
[modpacks page](/documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md)
feature doc lists every difference in its Design section.

## Code

- [Modpacks page](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/) — the built
  page the set was drawn for.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from the styles and
  fonts its html loads from the network.
- Used by: the page's feature doc, whose Design section links the set, and the page folder README.
- Rules: a set is kept as captured and never edited to match the built page; a new set gets its
  own folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a
  README; no screenshot of the built UI belongs here.
