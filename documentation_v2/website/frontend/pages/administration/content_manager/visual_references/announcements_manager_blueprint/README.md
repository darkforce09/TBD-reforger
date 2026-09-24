**Status:** live

# Announcements manager blueprint

Design-phase reference for the content manager page at `/admin/content`: a "Comms Link" reading
view of command announcements in a list-beside-detail split. It gives colour and layout context
and is not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/administration/content_manager/`.

## Contents

```text
documentation_v2/website/frontend/pages/administration/content_manager/visual_references/announcements_manager_blueprint/
├── announcements_manager_blueprint.html  the Stitch export of the announcements reader
└── announcements_manager_blueprint.png   its screenshot
```

## How it works

The blueprint shows a "Comms Link" list of announcements, each with a bracketed date, a title, a
summary and an unread dot, beside the open announcement: `[PINNED]` and `[MODPACK UPDATE]` tags,
an authority and timestamp line, the title, a body with a "Failure to Comply" callout and a
"Changelog Highlights" list, and a "Sync Status: Required" block with an "Initiate Download"
button.

The built page keeps only the split: its list holds the posts with a date, a title and a
"Published" or "Draft" badge, and its detail is the editor rather than a reader. It has no unread
markers, no pin control and no download block; members read announcements on the
[announcements page](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md).
The [content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md)
feature doc holds the full comparison.

## Code

- [Content manager page](/apps/website/frontend/src/v2/pages/administration/content_manager/) —
  the page this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the content manager feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
