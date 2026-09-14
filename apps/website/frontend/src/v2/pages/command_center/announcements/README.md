# Announcements Page (`/announcements`)

## Architecture
- **`page.rs`**: Master-detail responsive layout.
- **`article_feed.rs`**: Chronological list of dispatches with category filters.
- **`article_viewer.rs`**: Full markdown reader with author badge and timestamp.

## Not present in the legacy page
- **Category filters** — `article_feed.rs` orders pinned dispatches first and otherwise keeps the server's order; the filter icon in the list header is decorative.
- **Markdown rendering** — `article_viewer.rs` splits the body into blank-line separated paragraphs and renders each as a text node, escaped once. Inline markup shows as written; interpreting it would need a sanitiser on the write path first.
