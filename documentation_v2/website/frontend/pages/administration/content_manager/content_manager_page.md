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
- Entry: the `/admin/content` route renders `ContentManagerPage`
  (`apps/website/frontend/src/app_routes.rs`); `apps/website/frontend/src/router.rs` declares it
  for the `admin` tier, full-bleed, with the breadcrumb "Administration" › "Comms Broadcaster",
  and the sidebar lists it as "Comms Broadcaster"
  (`apps/website/frontend/src/v2/pages/navigation/nav_config.rs`).
- Related: the [content manager](/documentation_v2/glossary.md#content-manager) glossary entry;
  the [announcements page](/documentation_v2/website/frontend/pages/command_center/announcements/announcements_page.md),
  where members read what this page publishes; the [API](/documentation_v2/glossary.md#api)'s
  [community content domain](/apps/website/api_v2/src/community_content/README.md).

## Behaviour

1. The page starts the list fetch at once and renders inside `AdminGate`
   (`apps/website/frontend/src/v2/core/ui/gates.rs`): "Loading session…" while the session
   restores, a sign-in prompt for a signed-out viewer, and "Admin access required." below the
   `admin` [role](/documentation_v2/glossary.md#role).
2. The master column is headed "Comms Broadcaster" with a "New" button. The first successful
   fetch seeds the working set of posts once, and the first post opens; later fetches never
   overwrite it, so local edits survive. Until the list arrives, and when it is empty, the column
   reads "No announcements yet."; a failed fetch shows the server's sentence, else "Failed to load
   announcements", with a "Retry" button.
3. Each row shows a date (the publication time, else the last update, else the creation), the
   title ("Untitled Post" when blank) and a "Published" or "Draft" badge. With nothing selected the
   detail reads "Select a post or create a new one.".
4. "New" adds an unsaved post at the top of the list, titled "Untitled Post" in the Announcement
   category, and opens it. It exists only in the browser until it is published.
5. The editor holds the title ("Post Title"), the category (Announcement, SOP, Community Event,
   Modpack Update, Important), "Add Hero Image", a toolbar (Bold, Italic, Link, List, Image) that
   appends Markdown markers to the body, the body ("Start writing… Markdown supported."), the
   "Push to Discord" switch, on by default, and the buttons "Delete", "Save Draft" and
   "Publish & Broadcast". The body is plain Markdown text; the screen shows no preview.
6. "Add Hero Image" opens a file picker for JPEG, PNG or WebP. The upload answers a
   site-relative address, which the page makes absolute with the window's origin and stores as
   the post's hero image; it toasts "Hero image uploaded", or the server's sentence, else "Hero
   upload failed".
7. "Save Draft" keeps the editor's fields in the browser's working set, marks the post "Draft" in
   the list and toasts "Draft saved". It sends no request: the draft survives switching posts but
   not a reload.
8. "Publish & Broadcast" needs a title and a body ("Title and body are required") and a known
   category ("Unknown category — cannot publish"). An unsaved post is created on the server and
   takes the id the server gives it; a saved post is updated, and when the switch is on and the
   post was already published, the page then asks the server to push it to Discord again. The
   page toasts "Published & broadcast to Discord" when the switch is on and "Published" when it
   is off; a refusal shows the server's sentence, else "Publish failed".
9. Every publish sends `is_pinned` false; the page has no pin control.
10. "Delete" acts at once, without a confirmation. A saved post is archived on the server,
    leaves the list and toasts "Announcement archived"; an unsaved post is dropped with "Draft
    discarded" and no request.
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

The page README lists no calls, so the DTOs are named here. Server-side, in
`apps/website/api_v2/src/community_content/handlers/`:

- `GET /api/v1/cms/announcements?limit=100` (`list_cms_announcements` in
  `announcements_admin.rs`): read as `Paginated<Value>`. The API returns the drafts and published
  posts, neither archived nor deleted, pinned first and then most recently updated, at most 100.
- `POST /api/v1/cms/announcements` (`create_announcement`) with `{title, body, tag,
  thumbnail_url, is_pinned, push_to_discord, status: "published"}`: answers 201 with the post.
  The API refuses a blank title or body ("title and body are required"), an unknown tag ("invalid
  tag") or status ("invalid status"), and a hero address that is neither empty nor an absolute
  `http` or `https` URL. A published post with `push_to_discord` is pushed to the Discord webhook
  at once; a failed push does not fail the request and is recorded as `webhook.push_failed` at
  critical severity. The API records `announcement.create`.
- `PATCH /api/v1/cms/announcements/{id}` (`update_announcement`): the same body as a partial
  update. It pushes to Discord only when the request publishes the post now or the post was never
  pushed, and a failed push again does not fail the request.
- `POST /api/v1/cms/announcements/{id}/push-discord` (`push_announcement_discord` in
  `announcement_discord_push.rs`): pushes a published post again. It refuses a post that is not
  published (400 "only published announcements can be pushed to Discord"), a server without a
  webhook (400 "discord webhook not configured") and a failed push (502 "webhook push failed"),
  and answers `{pushed: true}`.
- `DELETE /api/v1/cms/announcements/{id}` (`delete_announcement`): sets the post's status to
  `archived` and answers 204; the row stays, so an archived post can be restored in the database.
- `POST /api/v1/cms/uploads` (`upload_image` in `media_upload.rs`): a multipart form with the
  field `file`. The API accepts at most 5 MB (413 "file exceeds 5MB") of `jpg`, `jpeg`, `png` or
  `webp` (415 otherwise) and answers 201 with `{url: "/uploads/<uuid>.<ext>"}`.

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
