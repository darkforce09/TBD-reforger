**Status:** live

# Event schedule page documentation

The feature documentation of the `/events` page in the
[operations](/documentation_v2/glossary.md#operations) section, where a member browses the
upcoming [events](/documentation_v2/glossary.md#event) beside the selected event's hub, with the
page's design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/operations/schedule/
├── event_schedule_page.md  the feature doc: the list, the embedded hub and the API's upcoming scope
└── visual_references/      the design-phase blueprint of the list beside an event's detail
```

## How it works

Read [event_schedule_page.md](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what the event list means in the
[API](/documentation_v2/glossary.md#api), and compares the built page with the blueprint in
`visual_references/`. The hub the detail column embeds is described once, in the
[event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
feature doc. The code folder's README lists the page's files, calls and states.

## Code

- [Event schedule page](/apps/website/frontend/src/v2/pages/operations/schedule/) — the route
  component `EventSchedulePage` and the event card.
- [Operations domain](/apps/website/api_v2/src/operations/) — the event list and the event hub
  the page reads.

## Boundaries

- Depends on: the feature doc template; the page code, the operations handlers and the ticket
  registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the page's in-code README, the operations pages README and the operations domain
  README, which link the feature doc; the [event](/documentation_v2/glossary.md#event) glossary
  entry; the feature doc template's worked sample; the event manager feature doc; the web app
  README's page table in `documentation_v2/website/frontend/`.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the API side of events and
  their hubs.
