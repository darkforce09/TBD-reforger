**Status:** live

# Content manager design references

The design references of the `/admin/content` page: two design-phase blueprint sets, kept for
colour and layout context rather than as implementation sources.

## Contents

```text
documentation_v2/website/frontend/pages/administration/content_manager/visual_references/
├── announcements_manager_blueprint/  an announcements reader: a dated list beside the open post
└── broadcast_editor_blueprint/       the distraction-free announcement editor with a Discord switch
```

## How it works

A set is a folder named after its subject and kind. It holds the Stitch export as an html file
and its screenshot as a png, both named after the set, and a README that says what the set shows
and how the built page differs. The broadcast editor blueprint is the closer of the two: the built
editor follows it, while the announcements manager blueprint lends only its list-beside-detail
split. The built UI is the Leptos code the Code section links; the
[content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md)
feature doc lists every difference in its Design section.

## Code

- [Content manager page](/apps/website/frontend/src/v2/pages/administration/content_manager/) —
  the built page the sets were drawn for.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from the styles and
  fonts its html loads from the network.
- Used by: the page's feature doc, whose Design section links both sets, and the page folder
  README.
- Rules: a set is kept as captured and never edited to match the built page; a new set gets its
  own folder, named `<subject>_<kind>` with the kind `blueprint`, `mockup` or `render`, and a
  README; no screenshot of the built UI belongs here.
