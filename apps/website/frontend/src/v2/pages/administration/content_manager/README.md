# Content & CMS Manager (`/admin/content`)

The unit's posts on the left, the one being written on the right.

## Architecture
- **`page.rs`**: route component — fetches the catalogue, seeds the working set from it once, and
  arranges the post list beside the editor.
- **`article_table.rs`**: the two state badges, and the row the master list renders per post.
- **`editor_form.rs`**: the title, category, body and hero fields, the markdown toolbar, the publish
  switch, and the save, publish, delete and re-push actions behind them.
- **`hero_upload.rs`**: the file picker, the multipart upload, and the absolute address it stores on
  the draft.
- **`doc.rs`**: the shape a post is edited as, the routes the screen calls, and the mapping in both
  directions between a category and the tag it is stored under.
- **`tests/content.rs`**: the routes held against the live router, the boot path, the failed-fetch
  behaviour, and the hero upload end to end.

## Not present in the legacy page
- **A live markdown preview**: the toolbar writes real markers into the body, and the body field is
  the only rendering of it on this screen.
- **A category for standing orders on the wire**: the stored tag set has no entry for one, so a
  standing order is stored under the closest tag that exists and reads back as an announcement.
