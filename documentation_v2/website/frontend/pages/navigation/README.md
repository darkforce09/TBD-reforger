**Status:** live

# Navigation frame documentation

The feature documentation of the frame every page renders in: the frame chosen per route, the
sidebar, the top bar with the account menu, the membership status panel and the not-found page,
with the top bar's design-phase reference. Developers and AI agents read it before changing the
frame or adding a route that needs its own layout.

## Contents

```text
documentation_v2/website/frontend/pages/navigation/
├── app_layout_and_navigation.md  the feature doc: frames, sidebar, top bar, membership panel, not-found
└── visual_references/            the design-phase reference of the top bar
```

## How it works

Read [app_layout_and_navigation.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md)
and covers the whole code folder in one doc, since the frame's parts render together on every
route. It links the in-code README for the link table, the calls and every interface text, gives
what each call means in the [API](/documentation_v2/glossary.md#api), and compares the built top
bar with the blueprint in `visual_references/`. The blueprint is a design-phase reference: it
draws the bar in a solid navy with a brighter blue, while the built bar uses the theme's
translucent surface.

| Page | Route and component | Label on screen | Feature doc |
|---|---|---|---|
| Not-found page | any path no route matches, `NotFoundPage` | "Sector Not Found" | [app_layout_and_navigation.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md#not-found-page) |
| Frame | every route, `AppLayout` | sidebar, top bar and membership panel | [app_layout_and_navigation.md](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md#frame-choice) |

## Code

- [Navigation frame](/apps/website/frontend/src/v2/pages/navigation/) — `AppLayout`, the sidebar
  and its link table, the top bar, the membership panel and the not-found page.
- [Route table](/apps/website/frontend/src/router.rs) — the layout flags and breadcrumbs the frame
  reads for each route.
- [Session and access](/apps/website/frontend/src/v2/core/auth/) — the session store the frame
  creates and the [role](/documentation_v2/glossary.md#role) checks the sidebar applies.

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
  the [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md);
  the [glossary](/documentation_v2/glossary.md); the frame code, the identity and access and
  administration handlers it calls, and the ticket registry in `.ai/tickets/`, which the feature
  doc is written from.
- Used by: the [frontend documentation](/documentation_v2/website/frontend/README.md) route table
  and the [page areas](/documentation_v2/website/frontend/pages/README.md) index; the frame's
  in-code README, which links the feature doc.
- Rules: `app_layout_and_navigation.md` keeps its name, which those links use; it stays within
  500 lines; design references live only in `visual_references/`, and no document holds a
  screenshot of the built UI.

## Related documentation

- [Account pages](/documentation_v2/website/frontend/pages/account/account_pages.md) — the
  sign-in pages the frame renders bare and the settings page its account menu opens.
- [Design tokens](/documentation_v2/design_system/design_tokens.md) — the theme the frame is drawn
  with.
