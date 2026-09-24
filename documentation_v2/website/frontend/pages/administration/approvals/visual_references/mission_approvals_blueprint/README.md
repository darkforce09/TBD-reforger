**Status:** live

# Mission approvals blueprint

Design-phase reference for the mission approvals page at `/admin/approvals`: a dossier in which a
reviewer reads one community-submitted mission and decides it. It gives colour and layout context
and is not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/administration/approvals/`.

## Contents

```text
documentation_v2/website/frontend/pages/administration/approvals/visual_references/mission_approvals_blueprint/
├── mission_approvals_blueprint.html  the Stitch export of the review dossier
└── mission_approvals_blueprint.png   its screenshot
```

## How it works

The blueprint shows the heading "Mission Approvals" with the line "Review and authorize
community-submitted missions for the live database.", a "Pending Queue (2)" column of cards, each
with a title, a date and the author's avatar, and the selected mission's dossier: title, author,
date and terrain, a map image, a "Tactical Briefing", tiles for BLUFOR slots, OPFOR and the
expected duration, a "Launch Tactical Planner for Deep Review" button, a "Review Activity" thread
with a comment box, and the buttons "Request Changes" and "Approve & Publish".

The built page differs: it has no heading of its own; its queue rows name the version and the
artifact under review instead of showing avatars; its drawer reviews an immutable artifact, with
its provenance and compile findings in place of the map and the settings tiles in place of the side
counts; a link to the read-only review workspace replaces the planner button; and three decisions,
two of them with a required text, replace the two buttons. The
[mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
feature doc holds the full comparison.

## Code

- [Mission approvals page](/apps/website/frontend/src/v2/pages/administration/approvals/) — the
  page this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and Google-hosted placeholder images, which the
  html loads when opened; the png needs nothing.
- Used by: the mission approvals feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
