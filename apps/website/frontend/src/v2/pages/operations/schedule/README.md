# Operations Schedule Page (`/events`)

Every scheduled operation in a master column, with the full hub of the selected one beside it.

## Architecture
- **`page.rs`**: the route component. Fetches `GET /events`, owns the selection signal and the
  memo that resolves it against the first row, keys a second fetch of `GET /events/:id` on that
  memo, and composes the split pane — including the three-state detail column that refuses to
  render one operation's hub under another operation's selection.
- **`upcoming_ops.rs`**: one operation card — local start time, lifecycle badge, mission and slot
  counts, countdown or lock marker, and the fill bar — plus the readers for the untyped event row.
- **`tests/schedule.rs`**: the guard that keeps the detail column routing briefings through the
  shared hub body, where the trim-aware empty rule lives.

## Not present in the legacy page
- **`calendar_strip.rs`**: there is no date picker. The page shows the list the API returns, in
  the order it returns it, with no week or month window to step through.
- **`past_operations.rs`**: there is no historical archive and no after-action link. The master
  column is one list of operations; a completed one carries its lifecycle badge and nothing more.
