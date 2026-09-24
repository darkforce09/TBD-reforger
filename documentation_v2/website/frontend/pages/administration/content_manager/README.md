**Status:** live

# Content manager page documentation

The feature documentation of the `/admin/content` page, labelled Comms Broadcaster on screen,
where administrators write, publish, push to Discord and archive announcements, with the page's
two design-phase references.

## Contents

```text
documentation_v2/website/frontend/pages/administration/content_manager/
├── content_manager_page.md  the feature doc: the post list, the editor, publishing and the API
└── visual_references/       design-phase blueprints of the editor and of an announcements reader
```

## How it works

Read [content_manager_page.md](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what each call means in the API, lists where the page's
wording and the API disagree, and compares the built page with the two blueprints in
`visual_references/`. Both are design-phase references: the built editor follows the broadcast
editor blueprint closely, and keeps only the list-beside-detail split of the announcements
manager blueprint. The code folder's README lists the page's files.

## Code

- [Content manager page](/apps/website/frontend/src/v2/pages/administration/content_manager/) —
  the route component `ContentManagerPage`, the post list, the editor and the hero upload.
- [Community content domain](/apps/website/api_v2/src/community_content/) — the announcement,
  Discord push and upload routes the page writes through.

## Boundaries

- Depends on: the feature doc template; the page code, the community content handlers and the
  ticket registry the feature doc is written from.
- Used by: the [content manager](/documentation_v2/glossary.md#content-manager) glossary entry, the
  page's in-code README and the community content domain README, which link the feature doc; the
  administration pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint sets stay as they
  were captured and are never edited to match the built page.

## Related documentation

- [Announcements page](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md)
  — the members' feed that shows what this page publishes.
