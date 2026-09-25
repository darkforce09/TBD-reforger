**Status:** live

# Event schedule design references

The design references of the `/events` page: one design-phase blueprint set, kept for colour and
layout context rather than as an implementation source.

## Contents

```text
documentation_v2/website/frontend/pages/operations/schedule/visual_references/
└── operations_schedule_blueprint/  a list of operations beside one operation's briefing and ORBAT
```

## How it works

A set is a folder named after its subject and kind. It holds the Stitch export as an html file
and its screenshot as a png, both named after the set, and a README that says what the set shows
and how the built page differs. The built UI is the Leptos code the Code section links; the
[event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
feature doc lists every difference in its Design section.

## Code

- [Event schedule page](/apps/website/frontend/src/v2/pages/operations/schedule/) — the built page
  the set was drawn for.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from the styles and
  fonts its html loads from the network.
- Used by: the page's feature doc, whose Design section links the set, and the page folder README.
- Rules: a set is kept as captured and never edited to match the built page; a new set gets its
  own folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a
  README; no screenshot of the built UI belongs here.
