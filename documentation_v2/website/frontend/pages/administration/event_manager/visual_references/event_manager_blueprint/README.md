**Status:** live

# Event manager blueprint

Design-phase reference for the [event manager](/documentation_v2/glossary.md#event-manager) page at
`/admin/events`: a tactical month calendar beside a form that schedules one operation. It gives
colour and layout context and is not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/administration/event_manager/`.

## Contents

```text
documentation_v2/website/frontend/pages/administration/event_manager/visual_references/event_manager_blueprint/
├── event_manager_blueprint.html  the Stitch export of the calendar and its form
└── event_manager_blueprint.png   its screenshot
```

## How it works

The blueprint shows a card headed "Event Manager" with the line "Schedule upcoming deployments and
toggle registration locks.", a month grid with weekday headers, and beside it the form "Schedule
Operation": the selected date, a start time, a "Mission Objective" select ("Select from Mission
Library..."), a registration status toggle, Open or Locked ("If locked, only admins can assign
personnel."), and the buttons "Publish Event" and "Delete Event".

The built page differs: its heading reads "Operations Calendar" with its own line; the form beside
the calendar becomes a day panel that lists the selected day's operations, with the schedule, edit
and delete actions in dialogs; an operation carries several
[missions](/documentation_v2/glossary.md#mission), a lifecycle state, a briefing, a banner and a
[slot](/documentation_v2/glossary.md#slot) ceiling; and an access sheet, which the blueprint does
not show, decides who may join. The
[event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md)
feature doc holds the full comparison.

## Code

- [Event manager page](/apps/website/frontend/src/v2/pages/administration/event_manager/) — the
  page this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the event manager feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
