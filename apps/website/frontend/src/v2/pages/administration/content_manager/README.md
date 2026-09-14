# Content & CMS Manager (`/admin/content`)

*Replaces the 1,474-line monolithic `pages/admin/content.rs` file.*

## Architecture
- **`page.rs`**: Main layout file.
- **`article_table.rs`**: Content list with Draft/Published state toggles.
- **`editor_form.rs`**: Markdown authoring panel with live preview.
