# Personnel Roster (`/admin/personnel`)

*Replaces the 1,294-line monolithic `pages/admin/personnel.rs` file.*

## Architecture
- **`page.rs`**: Main layout file.
- **`member_roster.rs`**: Member directory with callsigns, ranks, Discord IDs, and join dates.
- **`role_dialog.rs`**: Modal for promoting/demoting members and granting Admin/MissionMaker permissions.
