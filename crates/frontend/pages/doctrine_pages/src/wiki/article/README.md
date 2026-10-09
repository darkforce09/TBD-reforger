# Wiki article pane

The open manual in the wiki's detail pane: fetched on its own, shown with its stamps, category and
title, rendered or edited, and laid out beside its revision history.

## Contents

```text
crates/frontend/pages/doctrine_pages/src/wiki/article/
├── article_body.rs    the article area: an older revision on view, the editor, or the rendered blocks
├── article_header.rs  last-updated and revision stamps, category, title, the read/edit switch and "Save"
├── article_pane.rs    `article_pane`: the `GET /wiki/{slug}` fetch and the layout of the loaded manual
├── article_state.rs   `ArticleState`: the manual's reload counters, revision on view, busy flag, refusal
└── mod.rs             declares the pane and re-exports `article_pane` and `ArticleState`
```

## How it works

The route mounts `article_pane` afresh for each slug. The pane creates an `ArticleState`, a
`LocalResource` for `GET /wiki/{slug}` keyed on the state's article reload counter, and the two
history resources from `../revisions/`. While the first answer is pending it shows "Loading…"; a
failure shows its sentence; a loaded `WikiArticle` goes into a `StoredValue` read by the header,
the body and the revision panel. A reload keeps the previous article on screen until the new one
arrives.

`article_header` shows "Last updated <day>", "Revision <n>", the category and the title, and — for
the `is_admin` memo only — "[ READ ]"/"[ EDIT ]" and, while editing, "Save", which builds the draft
save and hands it to `../saving/`. `article_body` shows the revision on view when there is one,
the Markdown editor for an administrator in edit mode, and otherwise the blocks through
`../blocks/`. The editor reads the drafts untracked and writes each keystroke through
`updated_draft`, so the draft keeps the revision it started from. Reading a manual with a draft
shows the saved text under a notice, because the rendered blocks are always the server's parse
of the saved body.

## Boundaries

- Depends on: `../page_state.rs` (`WikiPageState`, `WikiMode`, `updated_draft`), `../blocks/`,
  `../revisions/` (the resources, the panel and the view), `../saving/` (the draft save, the
  submission and the refusal notice), `../display_text.rs`, `../api_paths.rs`,
  `frontend_transport` (`api_get`, `Fetched`, `frontend_api_dtos::wiki::WikiArticle`) and
  `frontend_session::AuthStore`.
- Used by: `../page.rs`, which mounts `article_pane` for the resolved slug; `../revisions/` and
  `../saving/`, which read and write `ArticleState`.
- Rules: the editing controls follow the reactive `is_admin` memo; a draft keeps its base
  revision; a manual is always fetched on its own, never read from the page list.

## Related documentation

- [Doctrine wiki page](/documentation/crates/frontend/pages/doctrine_pages/wiki/wiki_page.md)
  — reading, editing and the draft rules.
