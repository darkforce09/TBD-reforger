# Event Operations Manager (`/admin/events`)

*Replaces the 1,779-line monolithic `pages/admin/event_manager.rs` file.*

## Architecture
- **`page.rs`**: Main layout file (~250 LOC).
- **`event_table.rs`**: Schedule table with publish/cancel/archive actions.
- **`edit_dialog.rs`**: Modal wizard for scheduling an event, assigning missions, and configuring modpacks.
