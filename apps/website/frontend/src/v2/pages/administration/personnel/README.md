# Personnel Roster (`/admin/personnel`)

The community roster on the left, one member's dossier on the right, and the actions an
administrator can take on them.

## Architecture
- **`page.rs`**: route component — fetches `GET /admin/users`, owns the search box and the sort and
  filter controls, runs the Discord role resync, and arranges the two panes.
- **`member_roster.rs`**: the orders and subsets the header cycles through, the two pure passes that
  apply them, the table and its rows, and the initials badge a row falls back to.
- **`dossier.rs`**: the profile header, the four service readings, and the action buttons under
  them.
- **`role_dialog.rs`**: the inline role picker and the request behind it, and the ban and warning
  confirmations with the reason each requires.
- **`tests/personnel.rs`**: the route helpers held against the live router, the roster passes, and
  the required-reason rule.

## Not present in the legacy page
- **A rank filter**: the filter control cycles All, Active and Banned. Rank is a sort order, not a
  subset.
- **Join dates in the table**: the roster payload carries no join date; the columns are the member,
  their game character, their rank, their warning count and their status.
- **A promotion modal**: the role is changed with an inline picker in the dossier. The two dialogs
  on this screen are the ban and the warning, and both exist to collect a required reason.

## Related documentation

- [Personnel roster page](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
