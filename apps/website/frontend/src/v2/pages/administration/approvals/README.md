# Mission Approvals Queue (`/admin/approvals`)

The missions waiting on a reviewer, and the surface a decision is made on.

## Architecture
- **`page.rs`**: route component — fetches the pending queue, puts it behind the administrator
  gate, and owns which submission is open.
- **`submission_queue.rs`**: the queue heading with the backlog's total, one selectable row per
  submission, and the split that puts the queue beside the drawer.
- **`review_drawer.rs`**: the submission's header, the mission's own briefing and settings, the
  local scratch notes, and the action bar carrying the rejection reason.

## Not present in the legacy page
- **A diff validator and validation findings**: nothing on this screen inspects a submission's
  payload. The reviewer reads the mission's briefing and settings and decides.
- **Approved and rejected tabs**: the listing endpoint selects pending submissions only, so there
  is no second or third list to show.
- **Saved reviewer comments**: the notes box is local to the browser tab and says so on screen —
  there is no endpoint behind it. What the author is told is the rejection reason.
