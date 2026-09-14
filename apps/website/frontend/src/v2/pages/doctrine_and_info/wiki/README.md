# Doctrine Wiki (`/wiki`, `/wiki/:slug`)

Master-detail knowledgebase: a category-grouped index of manuals beside the manual being read.
Administrators can switch the reading pane to a raw-Markdown editor and save the page back.

## Architecture
- **`page.rs`**: route component — fetches `GET /wiki`, resolves the route slug against the
  returned list, owns the search text, the read/edit mode and the unsaved drafts, and composes
  the split view.
- **`category_nav.rs`**: the doctrine index — the search box and one labelled group per category,
  each row navigating to `/wiki/<slug>`.
- **`markdown_article.rs`**: the reading pane — the category, last-updated stamp and title, the
  rendered body or the raw-source editor, and the administrator's save button.
- **`markdown.rs`**: the Markdown renderer `markdown_article.rs` uses — headings, bullet lists,
  `> [!TYPE]` callouts, paragraphs, and bold / italic / code spans.
- **`helpers.rs`**: the two field reads over the still-untyped wiki payload.
- **`tests/wiki.rs`**: the pure helpers, and the guard that the editing affordances use the
  authenticated role check.

## Not present in the legacy page
- **In-page table of contents**: the article renders its headings inline; there is no contents
  rail beside them.
