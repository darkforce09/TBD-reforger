**Status:** live

# Operations schedule blueprint

Design-phase reference for the event schedule page at `/events`: a list of upcoming operations
beside the selected operation's briefing and [ORBAT](/documentation_v2/glossary.md#orbat). It
gives colour and layout context and is not an implementation source; the built UI is the Leptos
code under `apps/website/frontend/src/v2/pages/operations/schedule/`.

## Contents

```text
documentation_v2/website/frontend/pages/operations/schedule/visual_references/operations_schedule_blueprint/
├── operations_schedule_blueprint.html  the Stitch export of the schedule
└── operations_schedule_blueprint.png   its screenshot
```

## How it works

The blueprint shows "UPCOMING OPS" with the line "Select an operation to view telemetry" over
three cards, each with a date and time, a status ("OPEN", "PLANNED", "CLOSED"), a name, a terrain
and a "REGISTRATION" bar with a count. Beside them sit a banner image stamped
"CLASSIFIED // EYES ONLY" with the operation's name, a "MISSION BRIEFING", tabs for two
[missions](/documentation_v2/glossary.md#mission), and an "ORDER OF BATTLE" with faction tabs, a
squad list with fill counts, and slot rows with "ASSIGN" and "LOCKED".

The built page keeps the list beside the detail and differs: a card shows mission and
[slot](/documentation_v2/glossary.md#slot) counts, a countdown and a fill bar instead of a
terrain; the list holds upcoming events only; and the detail is the event hub, a hero without a
banner, the Places panel and one card per mission, each with its own briefing and ORBAT, instead
of mission tabs. The
[event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
feature doc holds the full comparison.

## Code

- [Event schedule page](/apps/website/frontend/src/v2/pages/operations/schedule/) — the page this
  set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the event schedule feature doc's Design section, the visual references README and the
  feature doc template's worked sample.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
