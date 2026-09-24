**Status:** live

# Event manager page documentation

The feature documentation of the `/admin/events` page, titled Operations Calendar on screen, where
administrators schedule and edit [events](/documentation_v2/glossary.md#event), attach
[missions](/documentation_v2/glossary.md#mission) and set who may join them, with the page's
design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/administration/event_manager/
├── event_manager_page.md  the feature doc: the calendar, the forms, the access sheet and the API
└── visual_references/     the design-phase blueprint of a calendar with a scheduling form
```

## How it works

Read [event_manager_page.md](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
its Behaviour runs through the calendar, the schedule and edit forms, deletion and the access sheet,
then lists where the page's wording and the [API](/documentation_v2/glossary.md#api) disagree; its
Data gives what each call means in the API; its Design compares the built page with the blueprint in
`visual_references/`. The blueprint is a design-phase reference: it shows one form beside the
calendar, while the built page schedules and edits in dialogs and adds the access sheet. The code
folder's README lists the page's files.

## Code

- [Event manager page](/apps/website/frontend/src/v2/pages/administration/event_manager/) — the
  route component `EventManagerPage`, the calendar, the forms and the access sheet.
- [Operations domain](/apps/website/api_v2/src/operations/) — the event, event mission, access,
  group, quota and waiting-list routes the page calls.

## Boundaries

- Depends on: the feature doc template; the page code and the operations handlers the feature doc
  is written from.
- Used by: the [event manager](/documentation_v2/glossary.md#event-manager) glossary entry and the
  page's in-code README, which link the feature doc; the administration pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Event administration evidence](/documentation_v2/website/api_v2/verification_evidence/event_administration.md)
  — the verification of event creation, editing, cancellation and deletion.
- [Eligibility and allocation evidence](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  — the verification of access policies, groups and reservation pools.
- [Reservation and attendance evidence](/documentation_v2/website/api_v2/verification_evidence/reservation_attendance.md)
  — the verification of reservations, the waiting list and attendance.
