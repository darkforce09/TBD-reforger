# Wiki handlers

The routes of the doctrine wiki: the navigation list, the article and the revision history, which
any signed-in member reads, and the save an administrator uses to create a page or record its next
revision.

## Contents

```text
apps/api/src/community_content/handlers/wiki_knowledgebase/
├── mod.rs         the module tree and the handler re-exports
├── page_store.rs  every wiki statement: page and revision reads, the save's lock, insert, update and revision copy
├── reads.rs       `GET /api/v1/wiki` and `GET /api/v1/wiki/{slug}`: the summaries and one article
├── revisions.rs   `GET /api/v1/wiki/{slug}/revisions` and `.../revisions/{revision}`: the history
└── save.rs        `PUT /api/v1/wiki/{slug}`: create a page or save its next revision
```

## How it works

The domain's `routes.rs` registers each route against its handler, relative to the `/api/v1`
nest.

| Route | Tier | Answer |
|---|---|---|
| `GET /wiki` | `AuthUser` | `{"data": [...]}` of summaries `{slug, category, title, icon?, nav_order, revision, updated_at}`, ordered by `nav_order`, `title`, `slug` |
| `GET /wiki/{slug}` | `AuthUser` | the article: the summary plus `id`, `updated_by?`, `body_md` and `blocks`; 404 for an unknown slug |
| `GET /wiki/{slug}/revisions?page&per_page` | `AuthUser` | `{items, page, per_page, total}`, newest first; each item `{revision, title, author_id?, created_at}` |
| `GET /wiki/{slug}/revisions/{revision}` | `AuthUser` | the page as that revision saved it, with its `blocks`; 404 when the page or revision is missing |
| `PUT /wiki/{slug}` | `AdminUser` | the saved article: 201 for a create, 200 for a new revision |

`blocks` come from `services::wiki_markup::read_markup`, so an article or revision always renders
safely whatever its stored markdown holds. The history's `page` defaults to 1 and `per_page` to
20; a `per_page` above 100 is served as 100, a value below 1 or not a number answers 400 (a query
string that does not decode answers in the error envelope through
`ApiError::from_query_rejection`), and a page past the end answers no items with the real total.
A revision that is not a number answers 400.

The save takes `{category, title, icon, nav_order, body_md, base_revision}`, every field required
and any other refused, and runs in this order:

1. The slug must match `^[a-z0-9-]{1,64}$`, else 400, before the body is read.
2. The body decodes through `ApiError::from_json_rejection`: 400 for a missing or unknown field,
   413 `request_too_large` over the request limit, 415 without a JSON content type.
3. `category`, `title` and `body_md` are non-empty and `base_revision` is `null` or at least 1,
   else 400; a `body_md` over 262 144 bytes answers 400 `wiki_body_too_large`.
4. Markup the service refuses answers 422 `wiki_markup_refused` with every finding
   `{line, code, detail}`.
5. One transaction locks the page row `FOR UPDATE`. `base_revision: null` inserts the page at
   revision 1, or answers 409 `wiki_revision_conflict` with `current_revision` when the slug
   exists; a number equal to the current revision updates the page to the next revision; any
   other number answers the same 409, and a number for a missing page 404.
6. The page's new content is copied into `wiki_page_revisions`, the audit line
   `wiki_page.created` or `wiki_page.updated` (target `wiki_page`, the slug) is appended through
   `administration::services::required_audit::append_actor_audit`, and the transaction commits.

The caller's Discord id is the editor (`updated_by`, `author_id`); an empty icon is stored as
null and left off the wire.

## Boundaries

- Depends on: `models::wiki` and `services::wiki_markup` in the domain;
  `administration::services::required_audit` for the audit line; `core` for the application
  state, the `AuthUser` and `AdminUser` extractors and `ApiError`; the `wiki_pages` and
  `wiki_page_revisions` tables of migration 0059.
- Used by: the domain's `routes.rs`; over HTTP, the wiki page under
  `apps/frontend/src/pages/doctrine_and_info/wiki/`.
- Rules: reads take `AuthUser` and the save `AdminUser`; every handler carries its `/// @route` tag
  (`cargo xtask verify route-tags`); the save locks the page row, then inserts the revision, then
  appends the audit row, all in one transaction; every nullable column is read through `COALESCE`
  or into an `Option` (`apps/api/tests/null_tolerance_select_scan.rs`);
  `apps/api/tests/community_content_reads.rs` holds the create, save, conflict,
  summary and history round trip against the contract.

## Related documentation

- [Wiki page](/documentation/apps/frontend/pages/doctrine_and_info/wiki/wiki_page.md) — the
  page that reads and writes these routes.
- [Administration and community content](/documentation/apps/api/verification_evidence/administration_and_content.md)
  — the wiki markup, revision and save design.
- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
