# Content manager page

The `/admin/content` page, the [content manager](/documentation_v2/glossary/a_to_f.md#content-manager),
headed "Comms Broadcaster": administrators write announcements in Markdown, give each a category and
a hero image, publish it to the members' announcements feed with an optional push to Discord, and
archive it. The page manages announcements only.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/content_manager/
├── article_table.rs  each post's list row: its date, its title and its published or draft badge
├── doc.rs            `Doc`, the post as edited; the routes; the category and tag mapping both ways
├── editor_form.rs    the editor: fields, Markdown toolbar, Discord switch, save, publish and delete
├── hero_upload.rs    the hero image picker, its multipart upload and the absolute address it stores
├── mod.rs            the module tree; re-exports `ContentManagerPage`
├── page.rs           `ContentManagerPage`: the list fetch, the working set and the split pane
└── tests/            unit tests for the mappings, the routes, the boot path and the wired requests
```

## How it works

`ContentManagerPage` starts the list fetch at once and renders inside `AdminGate`. The page keeps
a working set of posts: the first successful fetch seeds it once and opens the first post, and
later reads never overwrite it, so local edits survive. A failed fetch leaves the set unseeded and
shows its reason with a retry, never an empty catalogue. The list and the editor read the same
working set, so the two panes agree on every post's title and state.

A post created with "New" lives only in the browser, under a local id, until its first publish.
`is_server_id` tells a server id from a local one: publishing a local post creates it and keeps
the id the [API](/documentation_v2/glossary/a_to_f.md#api) mints, and publishing a saved post updates it;
when the Discord switch is on and the post was already published, the page then asks for the push
again, since an update pushes only on a first publish. "Save Draft" writes the fields back into
the working set and sends nothing. "Delete" acts without a confirmation: a saved post is archived
on the server, a local one is dropped. `category_tag` maps the five categories onto the API's four
tags, storing SOP under `update` like Announcement, so a saved SOP post reads back as Announcement
(`tag_category`). The toolbar appends Markdown markers to the body, and the page renders no
preview. `hero_upload.rs` makes the address an upload answers absolute with the window's origin,
because a publish refuses a relative one. Every request runs in the browser build only.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/content` | `ContentManagerPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` [role](/documentation_v2/glossary/n_to_z.md#role) only | full-bleed inside the navigation frame, over the topographic backdrop; breadcrumb Administration / Comms Broadcaster; sidebar entry "Comms Broadcaster" |

## Data

- `GET /api/v1/cms/announcements?limit=100`: read as `Paginated<Value>`; each row reads `id`,
  `title`, `body`, `tag`, `status`, `thumbnail_url`, `published_at`, `updated_at` and
  `created_at`, the date shown being the first of the last three that is present.
- `POST /api/v1/cms/announcements` for a local post, and `PATCH /api/v1/cms/announcements/{id}`
  for a saved one, each sending `title`, `body`, `tag`, `thumbnail_url`, `is_pinned: false`,
  `push_to_discord` and `status: "published"`; the create's answer gives the post its `id`.
- `POST /api/v1/cms/announcements/{id}/push-discord` with `{}`, after an update of a post already
  published while the switch is on.
- `DELETE /api/v1/cms/announcements/{id}`: archives a saved post.
- `POST /api/v1/cms/uploads`: a multipart form with the field `file`; the page reads the answer's
  `url`.
- The page reads the `AuthStore` context, the toast queue and the window's origin, and stores
  nothing in the browser; drafts live in memory until a reload.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| loading | "Loading…" |
| list failed | the API's sentence, else "Failed to load announcements", and "Retry" |
| no post | "Comms Broadcaster" and "New" over "No announcements yet." |
| list | per post its date, its title ("Untitled Post" when blank) and "Published" or "Draft" |
| nothing selected | "Select a post or create a new one." |
| editor | "Post Title", the category ("Announcement", "SOP", "Community Event", "Modpack Update", "Important"), "Add Hero Image", the toolbar "Bold", "Italic", "Link", "List" and "Image", the body "Start writing… Markdown supported.", the "Push to Discord" switch, on at first, and "Delete", "Save Draft" and "Publish & Broadcast" |
| saved or discarded | "Draft saved", "Draft discarded" or "Announcement archived"; "Delete failed" or the API's sentence on failure |
| publish refused | "Title and body are required", "Unknown category — cannot publish"; the API's sentence, else "Publish failed"; "publish returned no id" when a create answers no id |
| published | "Published & broadcast to Discord" with the switch on, "Published" with it off |
| hero upload | "Hero image uploaded"; the API's sentence, else "Hero upload failed"; "Upload returned no url", "Hero image upload failed — no document" or "Hero image upload failed — could not open file picker" |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `api_post`, `api_post_ok`, `api_patch`,
  `api_delete`, `api_upload_file`, `api_error_message`, `Paginated`), `crate::v2::core::auth`
  (`AuthStore`) and `crate::v2::core::ui` (`AdminGate`, `SplitPane`, `SplitPaneEmpty`,
  `ListDetailItem`, `MaterialIcon`, the toast queue); over HTTP, the
  announcement and upload routes of the
  [community content](/documentation_v2/glossary/a_to_f.md#community-content) domain.
- Used by: the `/admin/content` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Comms Broadcaster" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `content_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the page's sources for its
  tests; the DOM oracle's `content` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the page boots from the announcements list, never from built-in posts
  (`content_boots_from_cms_list_not_mock_docs`), and a failed list is never seeded as an empty one
  (`content_list_error_does_not_seed_as_empty_success`); every category, SOP included, maps to a
  tag the API accepts (`category_tag_covers_all_ui_categories_including_sop`); the paths are held
  against the API's route table (`cms_paths_match_axum_routes`); publish, delete, push and the hero
  upload reach their routes (`publish_edit_delete_push_are_wired_no_fake_toasts`,
  `hero_multipart_upload_is_wired_not_stubbed`), all in `tests/content.rs`.

## Related documentation

- [Content manager page](/documentation_v2/website/frontend/pages/administration/content_manager/content_manager_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the
  announcement and upload routes.
