**Status:** live

# Doctrine wiki page documentation

The feature documentation of the `/wiki` and `/wiki/:slug` page, where members read the
community's standard operating procedures and manuals and administrators edit them.

## Contents

```text
documentation_v2/website/frontend/pages/doctrine_and_info/wiki/
└── wiki_page.md  the feature doc: the index, the Markdown subset, editing and the API
```

## Code

- [Doctrine wiki page](/apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/) — the route
  component `WikiPage`, the index, the reading pane and the renderer.
- [Community content domain](/apps/website/api_v2/src/community_content/) — the wiki list and
  write routes the page calls.

## Boundaries

- Depends on: the feature doc template; the page code, the wiki handlers and the ticket registry
  the feature doc is written from.
- Used by: the in-code READMEs of the wiki and of the doctrine and info pages, which link the
  feature doc; the doctrine and info pages README.
- Rules: the feature doc keeps its name, which those links use; the page has no design set, and
  its Design section compares it with the archived platform spec instead.

## Related documentation

- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the API side
  of the wiki.
