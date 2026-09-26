**Status:** live

# ORBAT selection page documentation

The feature documentation of the `/events/:id/missions/:emid/orbat` page, where one
[mission](/documentation_v2/glossary/g_to_m.md#mission)'s [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat)
slotting stands on a page of its own for direct links.

## Contents

```text
documentation_v2/website/frontend/pages/operations/orbat_selection/
└── orbat_selection_page.md  the feature doc: the mission lookup, the mounted selector, stale links
```

## How it works

Read [orbat_selection_page.md](/documentation_v2/website/frontend/pages/operations/orbat_selection/orbat_selection_page.md).
It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
covers what the page adds around the selector: the mission lookup, the header and what a stale
link shows. The selector itself is described once, in the
[event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
feature doc. No blueprint set exists for this page. The code folder's README lists the page's
file, calls and states.

## Code

- [ORBAT selection page](/apps/website/frontend/src/v2/pages/operations/orbat_selection/) — the
  route component `OrbatSelectionPage`.
- [Event hub page](/apps/website/frontend/src/v2/pages/operations/event_detail/) — the
  `OrbatSelector` and `MissionStanding` the page mounts.

## Boundaries

- Depends on: the feature doc template; the page code, the event hub feature doc, the operations
  handlers and the ticket registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the page's in-code README, the operations pages README and the event hub and
  deployments feature docs, which link the feature doc.
- Rules: the feature doc keeps its name, which those links use; it never repeats the selector's
  behaviour, which stays in the event hub feature doc.
