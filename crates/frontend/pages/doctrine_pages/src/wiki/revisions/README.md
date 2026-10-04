# Wiki revision history

The open manual's revision history: the paged list beside the article, an older revision shown in
place of the current text, and the administrator's restore behind a confirmation.

## Contents

```text
crates/frontend/pages/doctrine_pages/src/wiki/revisions/
├── mod.rs                declares the history modules and re-exports what the article pane mounts
├── revision_fetches.rs   the resources for one page of the history and for the revision on view
├── revision_list.rs      the "Revision history" panel: rows, the current mark and the pager
└── revision_view.rs      an older revision's banner and blocks, and the restore confirmation
```

## How it works

The article pane creates both resources when a manual opens. The list resource fetches
`GET /wiki/{slug}/revisions?page=<n>&per_page=10` and refetches when the article's history page
or history reload counter changes; the revision resource fetches
`GET /wiki/{slug}/revisions/{revision}` whenever the article's `viewing` names a revision, and
resolves to `None` otherwise.

`revision_list` renders one row per `WikiRevisionSummary` — "Revision <n>", a "current" mark on the
article's revision, the title, and "saved <day> by <editor>" — and a pager reading the page numbers
the server answered ("Page <p> of <q>", "Newer", "Older"). Choosing a row puts that revision on
view; choosing the current one returns to the current text. `revision_view` shows a fetched
revision only when it is the one on view, with "Back to the current revision" and, for an
administrator on any revision but the current one, "Restore this revision". The confirmation
("Restore this revision?") builds the restore request — the revision's fields over the article's
current revision — and hands it to `../saving/`, which reports a refusal like any save.

## Boundaries

- Depends on: `../article/` (`ArticleState`), `../api_paths.rs`, `../blocks/`,
  `../display_text.rs`, `../page_state.rs`, `../saving/` (`restore_request`, `submit_save`),
  `frontend_transport` (`api_get`, `Fetched`, `frontend_api_dtos::wiki`), `frontend_session::AuthStore`
  and `frontend_ui::Dialog`.
- Used by: `../article/article_pane.rs` (the resources and the panel) and
  `../article/article_body.rs` (the view).
- Rules: a restore names the article's current revision as its base
  (`wiki_restore_request_sends_the_old_revision_over_the_current_one` in
  `../saving/tests/save_requests.rs`); the history is asked for from page 1 onward
  (`wiki_paths_never_ask_for_a_history_page_before_the_first`); only an administrator is offered a
  restore.

## Related documentation

- [Doctrine wiki page](/documentation/crates/frontend/pages/doctrine_pages/wiki/wiki_page.md)
  — the history and the restore.
