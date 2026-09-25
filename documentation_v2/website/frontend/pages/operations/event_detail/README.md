**Status:** live

# Event hub page documentation

The feature documentation of the `/events/:id` page, one [event](/documentation_v2/glossary.md#event)'s
hub with its places, [mission](/documentation_v2/glossary.md#mission) dossiers and inline
[ORBAT](/documentation_v2/glossary.md#orbat) slotting, which the schedule also embeds.

## Contents

```text
documentation_v2/website/frontend/pages/operations/event_detail/
└── event_hub_page.md  the feature doc: the hub, the places, the dossiers, slotting and the API
```

## How it works

Read [event_hub_page.md](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md).
It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md): it
quotes the hub's interface text, gives what each registration, squad and seat call means in the
[API](/documentation_v2/glossary.md#api), lists where the page and the API disagree, and compares
the hub with the design references it has. The slotting it describes is the same one the event
schedule and the ORBAT selection page show, so their feature docs link here rather than repeat
it. No blueprint set exists for this page; the event schedule's blueprint depicts the hub in its
detail column. The code folder's README and its registration access README list the files, calls
and states.

## Code

- [Event hub page](/apps/website/frontend/src/v2/pages/operations/event_detail/) — the route
  component `EventHubPage`, the hub body `event_hub_view` and the selector `OrbatSelector`.
- [Operations domain](/apps/website/api_v2/src/operations/) — the hub, the ORBAT, registrations,
  squad holds, seat assignment, the waiting list and the member directory.

## Boundaries

- Depends on: the feature doc template; the page code, the operations handlers and reservation
  services, and the ticket registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the page's in-code README and its registration access README, the operations pages
  README, the operations domain and handlers READMEs, which link the feature doc; the event
  schedule, ORBAT selection and event manager feature docs; the feature doc template's worked
  sample; the web app README's page table in `documentation_v2/website/frontend/`.
- Rules: the feature doc keeps its name, which those links use; a design set for this page, when
  one exists, goes into a `visual_references/` folder here.

## Related documentation

- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the API side of the hub
  and its slotting.
- [Event reservation services](/apps/website/api_v2/src/operations/services/event_reservations/README.md)
  — the pools, claims, holds and promotion rules behind registration.
