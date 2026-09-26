**Status:** live

# Content manager page

The `/admin/content` page, labelled "Comms Broadcaster" on screen: administrators write
announcements in Markdown, give them a category and a hero image, publish them to the members'
announcements feed with an optional push to Discord, and archive them. The page manages
announcements only; the doctrine wiki has no editor here.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/content_manager/`](/apps/website/frontend/src/v2/pages/administration/content_manager/):
  `page.rs` holds the route component `ContentManagerPage`, the list fetch and the working set
  of posts; `article_table.rs` the list rows and their badges; `editor_form.rs` the editor and
  its save, publish and delete actions; `hero_upload.rs` the hero image picker and upload;
  `doc.rs` the post shape, the routes and the category-to-tag mapping. The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/content_manager/README.md)
  describes each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/administration/content_manager/README.md#routes).
- Related: the [content manager](/documentation_v2/glossary/a_to_f.md#content-manager) glossary entry;
  the [announcements page](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md),
  where members read what this page publishes; the [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [community content domain](/apps/website/api_v2/src/community_content/README.md).

## Behaviour

1. The page starts the list fetch at once and renders inside `AdminGate`
   (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows the session and access states
   of the README's
   [States](/apps/website/frontend/src/v2/pages/administration/content_manager/README.md#states)
   in place of the page until a signed-in viewer holds the `admin`
   [role](/documentation_v2/glossary/n_to_z.md#role).
2. The master column holds the heading and a "New" button. The first successful fetch seeds the
   working set of posts once, and the first post opens; later fetches never overwrite it, so
   local edits survive. A failed fetch shows its reason with a "Retry" button, and an empty
   working set says there is nothing yet.
3. Each row shows a date (the publication time, else the last update, else the creation), the
   title and a published or draft badge.
4. "New" adds an unsaved post at the top of the list, untitled in the Announcement category, and
   opens it. It exists only in the browser until it is published.
5. The editor holds the title, the category (Announcement, SOP, Community Event, Modpack Update,
   Important), "Add Hero Image", a toolbar that appends Markdown markers to the body, the body,
   the "Push to Discord" switch, on by default, and the buttons "Delete", "Save Draft" and
   "Publish & Broadcast". The body is plain Markdown text; the screen shows no preview.
6. "Add Hero Image" opens a file picker for JPEG, PNG or WebP. The upload answers a
   site-relative address, which the page makes absolute with the window's origin and stores as
   the post's hero image.
7. "Save Draft" keeps the editor's fields in the browser's working set and marks the post a draft
   in the list. It sends no request: the draft survives switching posts but not a reload.
8. "Publish & Broadcast" needs a title, a body and a known category. An unsaved post is created
   on the server and takes the id the server gives it; a saved post is updated, and when the
   switch is on and the post was already published, the page then asks the server to push it to
   Discord again. The success toast follows the switch; a refusal shows the server's sentence.
9. Every publish sends `is_pinned` false; the page has no pin control.
10. "Delete" acts at once, without a confirmation. A saved post is archived on the server and
    leaves the list; an unsaved post is dropped with no request. Every toast is in the README's
    [States](/apps/website/frontend/src/v2/pages/administration/content_manager/README.md#states).
11. The SOP category is stored under the tag `update`, the one Announcement uses, because the
    stored tag set has no SOP entry; a saved SOP post reads back as Announcement.

### Known discrepancies

- Every label on screen (the sidebar entry, the breadcrumb and the heading) reads "Comms
  Broadcaster", while the route, the component and the code name the page the content manager,
  and the page manages announcements only.
- "Save Draft" persists nothing and marks a published post "Draft" in the list although the
  server still holds it published (`editor_form.rs` in the code folder, the `save_draft` handler).
- "Published & broadcast to Discord" follows the switch, not the push: on a create or an update
  the API publishes even when the Discord push fails, recording the failure only in the audit
  trail, so the toast can claim a broadcast that did not happen. Only the explicit re-push of an
  already published post reports a failed push.
- The list is ordered pinned first, but every publish from this page sends `is_pinned` false, so
  publishing a pinned post unpins it.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/administration/content_manager/README.md#data)
lists each call with the DTO or body it reads or sends. Server-side, in
`apps/website/api_v2/src/community_content/handlers/`:

- `GET /api/v1/cms/announcements?limit=100` (`list_cms_announcements` in
  `announcements_admin.rs`): the API returns the drafts and published posts, neither archived nor
  deleted, pinned first and then most recently updated, at most 100.
- `POST /api/v1/cms/announcements` (`create_announcement`): answers 201 with the post.
  The API refuses a blank title or body ("title and body are required"), an unknown tag ("invalid
  tag") or status ("invalid status"), and a hero address that is neither empty nor an absolute
  `http` or `https` URL. A published post with `push_to_discord` is pushed to the Discord webhook
  at once; a failed push does not fail the request and is recorded as `webhook.push_failed` at
  critical severity. The API records `announcement.create`.
- `PATCH /api/v1/cms/announcements/{id}` (`update_announcement`): a partial update with the
  create's fields. It pushes to Discord only when the request publishes the post now or the post was never
  pushed, and a failed push again does not fail the request.
- `POST /api/v1/cms/announcements/{id}/push-discord` (`push_announcement_discord` in
  `announcement_discord_push.rs`): pushes a published post again. It refuses a post that is not
  published (400 "only published announcements can be pushed to Discord"), a server without a
  webhook (400 "discord webhook not configured") and a failed push (502 "webhook push failed"),
  and answers `{pushed: true}`.
- `DELETE /api/v1/cms/announcements/{id}` (`delete_announcement`): sets the post's status to
  `archived` and answers 204; the row stays, so an archived post can be restored in the database.
- `POST /api/v1/cms/uploads` (`upload_image` in `media_upload.rs`): the API accepts at most 5 MB
  (413 "file exceeds 5MB") of `jpg`, `jpeg`, `png` or `webp` (415 otherwise) and answers 201 with
  `{url: "/uploads/<uuid>.<ext>"}`.

## Design

- A full-bleed `SplitPane`: a 20rem master column with the post list, and the editor as the
  detail, its title, category and hero controls on top, the body filling the height and the
  switch and buttons in a footer bar.
- Design targets, both design-phase references:
  - the [broadcast editor blueprint](/documentation_v2/website/frontend/pages/administration/content_manager/visual_references/broadcast_editor_blueprint/README.md),
    the editor. The built editor follows it closely and differs in these ways: it sits beside a
    post list; its heading has no subtitle; its toolbar has no underline, numbered-list or code
    tools; it adds the Announcement and SOP categories, names Event "Community Event" and adds a
    "Delete" button; the switch has no "Send embed to #announcements" caption; and a draft is not
    saved on the server;
  - the [announcements manager blueprint](/documentation_v2/website/frontend/pages/administration/content_manager/visual_references/announcements_manager_blueprint/README.md),
    a "Comms Link" reading view: a dated list with unread dots, `[PINNED]` and category tags, a
    hero image, a callout and a "Sync Status: Required" block with "Initiate Download". The page
    keeps only its list-beside-detail split, with the editor as the detail; it has no reading view,
    no unread markers and no pin control.
- The archived platform spec's
  [Content Manager section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#11-content-manager)
  asks for tabs between announcements and the wiki and a rich-text editor; the page has neither.

## Open work

- [T-087 — CMS rich text editor](/.ai/tickets/T-087.toml) (deferred, no plan): the body becomes a
  rich-text editor in place of Markdown text with a marker toolbar.
- [T-1018 — Fix content manager Save Draft and Discord broadcast toast](/.ai/tickets/T-1018.toml)
  (idea, no plan): "Save Draft" saves the post as a draft instead of only toasting "Draft saved",
  and the publish toast claims a Discord broadcast only when the push succeeded.
- [T-1028 — Fix re-publishing a pinned announcement unpinning it](/.ai/tickets/T-1028.toml)
  (idea, no plan): publishing an existing post keeps its pin instead of sending
  `is_pinned: false`.

## Decisions

- A post is created once and updated after: a saved post is always sent as an update, since
  posting it twice would leave two copies.
- An update pushes to Discord only on a first publish, so the page re-pushes an already published
  post through its own route when the switch is on.
- The working set is seeded once: a later read of the list cannot overwrite a local draft or an
  edit that was just published.
- Deleting archives: the row stays in the database, and both the members' feed (published posts
  only) and this list (drafts and published posts) drop it.
