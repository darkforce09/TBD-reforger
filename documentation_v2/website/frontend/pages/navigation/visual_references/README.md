**Status:** live

# Navigation frame design references

The design references of the navigation frame: one design-phase set for the top bar, kept for
colour and layout context rather than as an implementation source.

## Contents

```text
documentation_v2/website/frontend/pages/navigation/visual_references/
└── topbar_blueprint/  the top bar: breadcrumb, identity pill, avatar and open account menu
```

## How it works

A set is a folder named after its subject and kind. This set holds only the Stitch design brief
with its token front matter, `design_tokens.md`, and a README that says what the set shows and how
the built bar differs; no Stitch export or screenshot of it exists. The sidebar has no set. The
built UI is the Leptos code the Code section links; the
[app layout and navigation](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md)
feature doc lists every difference in its Design section.

## Code

- [Navigation frame](/apps/website/frontend/src/v2/pages/navigation/) — the built frame; the set
  was drawn for `top_nav.rs`.

## Boundaries

- Depends on: nothing in the repository; the set is self-contained.
- Used by: the feature doc, whose Design section links the set, and the navigation documentation
  README.
- Rules: a set is kept as captured and never edited to match the built frame; a new set gets its
  own folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a
  README; no screenshot of the built UI belongs here.
