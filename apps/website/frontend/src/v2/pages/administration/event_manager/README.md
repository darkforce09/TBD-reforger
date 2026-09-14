# Event Operations Manager (`/admin/events`)

A month calendar of the unit's operations, the panel of whatever day is selected, and the two
frosted forms that schedule a new operation or rewrite an existing one.

## Architecture
- **`page.rs`**: route component — builds the screen's state, puts it behind the administrator
  gate, and composes the calendar and the four dialogs in the order the stacking depends on.
- **`state.rs`**: the one copyable handle every panel reads — the calendar's position, both forms'
  fields, the in-flight flags, the three fetches, and the day grouping derived from them.
- **`event_table.rs`**: the page heading, the month grid with one cell per day, the selected day's
  operations, and the two controls that open the edit form and arm the delete.
- **`schedule_dialog.rs`**: the Schedule Operation form and the two-step publish behind it.
- **`edit_dialog.rs`**: the Edit Operation form and the save that sends only what changed.
- **`mission_picker.rs`**: the two places missions are attached — the schedule form's staging list,
  and the edit form's live roster with its detach control.
- **`confirm_dialogs.rs`**: the delete confirmation and the detach confirmation, with the two
  requests they guard.
- **`dates.rs`**: the date arithmetic — local day keys, the values the date and time fields want,
  and the instant a wall clock reads back as.
- **`lifecycle.rs`**: the six operation states, the moves the server will accept between them, the
  badge each is shown with, and the delete confirmation's copy.
- **`tests/event_manager.rs`**: the edit form's attach and clear wiring, and the delete copy held
  against what the endpoint actually does.

## Not present in the legacy page
- **A Discord announcement control**: neither form has one. Publishing an operation sends nothing
  to chat from this screen.
- **Modpack configuration**: an operation carries no modpack. The forms cover time, name, briefing,
  banner, slot ceiling, attached missions, lifecycle state and registration, and nothing else.
- **Publish / cancel / archive actions on the table**: the day panel offers Edit and Delete. The
  lifecycle state is set in the edit form's picker, which offers only the moves the server accepts.
