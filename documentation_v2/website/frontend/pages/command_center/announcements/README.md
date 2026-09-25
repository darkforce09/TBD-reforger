**Status:** live

# Announcements page documentation

The feature documentation of the `/announcements` and `/announcements/:id` pages in the
[command center](/documentation_v2/glossary.md#command-center), where a member reads the unit's
published announcements in a list beside a reading pane.

## Contents

```text
documentation_v2/website/frontend/pages/command_center/announcements/
└── announcements_page.md  the feature doc: the list, opening one by address, the reader and the API
```

## How it works

Read [announcements_page.md](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md).
It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md) and
covers both routes, since `/announcements/:id` is the same page with one announcement open. It
quotes the page's interface text, gives what the feed means in the
[API](/documentation_v2/glossary.md#api), and compares the page with the archived design spec,
since no blueprint set exists for it. The code folder's README lists the page's files, its call
and its states.

## Code

- [Announcements page](/apps/website/frontend/src/v2/pages/command_center/announcements/) — the
  route component `AnnouncementsPage`, the list and the reading pane.
- [Community content domain](/apps/website/api_v2/src/community_content/) — the announcement feed
  the page reads and the CMS routes that write it.

## Boundaries

- Depends on: the feature doc template; the page code, the announcement handlers and the ticket
  registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the page's in-code README and the command center pages README, which link the feature
  doc; the content manager's feature doc, README and announcements manager blueprint; the web app
  README's page table in `documentation_v2/website/frontend/`.
- Rules: the feature doc keeps its name, which those links use; a design set for this page, when
  one exists, goes into a `visual_references/` folder here.

## Related documentation

- [Content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md)
  — where announcements are written, published and pinned.
- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the API side
  of the feed.
