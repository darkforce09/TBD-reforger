**Status:** live

# Broadcast editor blueprint

Design-phase reference for the content manager page at `/admin/content`: the distraction-free
"Comms Broadcaster" editor in which an administrator writes one announcement. It gives colour and
layout context and is not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/administration/content_manager/`.

## Contents

```text
documentation_v2/website/frontend/pages/administration/content_manager/visual_references/broadcast_editor_blueprint/
├── broadcast_editor_blueprint.html  the Stitch export of the editor
└── broadcast_editor_blueprint.png   its screenshot
```

## How it works

The blueprint shows the heading "Comms Broadcaster" with the line "Create and distribute
operational updates across the network.", a title field ("Enter Announcement Title..."), a
category select (Event, Modpack Update, Important), "Add Hero Image", a formatting toolbar (bold,
italic, underline, link, image, lists, code), the body ("Draft your briefing here..."), a "Push to
Discord" switch captioned "Send embed to #announcements", and the buttons "Save Draft" and
"Publish & Broadcast".

The built editor follows it closely and differs in these ways: it sits beside a list of posts;
its toolbar has no underline, numbered-list or code tool; its categories add Announcement and SOP
and name Event "Community Event"; it adds a "Delete" button; the switch has no caption; and "Save
Draft" keeps the draft in the browser only. The
[content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md)
feature doc holds the full comparison.

## Code

- [Content manager page](/apps/website/frontend/src/v2/pages/administration/content_manager/) —
  the page this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the content manager feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
