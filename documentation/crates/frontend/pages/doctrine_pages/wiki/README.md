**Status:** live

# Doctrine wiki page documentation

The feature documentation of the `/wiki` and `/wiki/:slug` page, where members read the
community's standard operating procedures and manuals and administrators edit and restore them.

## Contents

```text
documentation/crates/frontend/pages/doctrine_pages/wiki/
└── wiki_page.md  the feature doc: the index, rendering, the history, editing, restoring and the API
```

## Code

- [Doctrine wiki page](/crates/frontend/pages/doctrine_pages/src/wiki/) — the route
  component `WikiPage`, the index, the article pane, the block renderer, the revision history and
  the save path.
- [Community content domain](/crates/api/api_community_content/src/) — the wiki list, article,
  revision and write routes the page calls, and the markup service.

## Boundaries

- Depends on: the feature doc template; the page code, the wiki handlers and the ticket registry
  the feature doc is written from.
- Used by: the in-code READMEs of the wiki and of the doctrine and info pages, which link the
  feature doc; the doctrine and info pages README.
- Rules: the feature doc keeps its name, which those links use; the page has no design set, and
  its Design section compares it with the archived platform spec instead.

## Related documentation

- [Community content domain](/crates/api/api_community_content/src/README.md) — the API side
  of the wiki.
