# Personnel roster page

The `/admin/personnel` page, [personnel](/documentation_v2/glossary/n_to_z.md#personnel), titled
"Personnel Roster": administrators search the member roster, open one member's dossier beside it,
ban, unban or warn a member, and resync every member's [role](/documentation_v2/glossary/n_to_z.md#role)
from Discord. A role follows the member's Discord roles, so the dossier explains it and never sets
it.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/personnel/
├── dossier.rs        one member's profile, four readings and the action buttons
├── member_roster.rs  the sort and filter modes, the passes that apply them, and the roster table
├── mod.rs            the module tree; re-exports `PersonnelRosterPage`
├── page.rs           `PersonnelRosterPage`: the gate, the roster fetch, the header and the resync
├── role_dialog.rs    the role note, and the ban and warning dialogs with their required reason
└── tests/            unit tests for the routes, the reason rule, the roster passes and the role note
```

## How it works

`PersonnelRosterPage` renders `PersonnelInner` inside `AdminGate`. The inner component keys one
roster fetch on the search text, so every keystroke fetches again, and sends the text as `q` only
when it is not empty. The sort and filter controls cycle through their modes and rearrange the
loaded rows in the browser (`apply_roster_filter`, then `apply_roster_sort`), never fetching
again: the roster route takes a search term and nothing else. Every order breaks ties on a second
key, so rows that compare equal keep their places; the role order compares the role's wire value.

Both panes read the same fetched page, so the dossier never shows a member the table no longer
lists. A row click sets `selected_id`, and `dossier` builds the pane from that row, seeding `role`,
`banned` and `warnings` signals so a ban, an unban or a warning shows at once; each also fetches
the roster again. The ban and warning dialogs need a reason: `classify_ban_reason` trims it, and
the confirm button stays disabled while it is blank (`reason_confirm_enabled`). An unban acts at
once, without a dialog. "Edit Roles" opens a note that sends nothing: the website sets no role, so
the page has no role picker and never calls the role route. "Sync Roles" posts the resync and
reports the `updated` count it answers; an answer without the count is reported as unexpected,
never as a completed sync. The page asks for no further page, so it lists the first page the
[API](/documentation_v2/glossary/a_to_f.md#api) returns. Every request runs in the browser build only; a
native build renders the failure branch.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/personnel` | `PersonnelRosterPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` role only | full-bleed inside the navigation frame; breadcrumb Administration / Personnel Roster; sidebar entry "Personnel Roster" |

## Data

- `GET /api/v1/admin/users`, or `GET /api/v1/admin/users?q=<text>` for a search: read as
  `Paginated<AdminUserRow>`; the table and the dossier read `discord_id`, `username`,
  `discord_handle`, `arma_id`, `arma_character`, `role`, `is_banned`, `warnings` and
  `total_deployments`. The page sends no `limit` or `offset`, so it reads the first page.
- `POST /api/v1/admin/users/{discordId}/ban` with `{"reason": <text>}`, and `DELETE` on the same
  path to unban; the page reads only whether each succeeded.
- `POST /api/v1/admin/users/{discordId}/warnings` with `{"reason": <text>}`; likewise.
- `POST /api/v1/admin/roles/sync` with `{}`: the page reads the answer's `updated` count.
- The page reads the `AuthStore` context and the toast queue, and stores nothing in the browser.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| header | "Personnel Roster", "Sync Roles" ("Syncing…" while it runs), the sort control ("Sort: Name", "Sort: Warnings", "Sort: Role", "Sort: Banned"), the filter control ("Filter: All", "Filter: Active", "Filter: Banned") and the search field "Search Discord ID or Arma Name…" |
| roster loading | "Loading…" |
| roster failed | "Failed to load data." |
| no member | "No users found." |
| roster | the columns "User", "Arma Character", "Rank", "Warnings" and "Status"; per member an initials badge with the Discord handle, else the username; the character, else the Arma id, else "Unlinked"; the role in capitals; the warning count, yellow above zero; and "Active" or "Banned" |
| no dossier | "Select personnel to view dossier" |
| dossier | the initials badge, the name, the Discord id and the Arma identity, or "Unlinked Arma identity"; the readings "Deployments", "Current Rank", "Warnings" and "Status"; "Edit Roles", "Issue Warning" ("Issuing…"), and "Ban Personnel" ("Banning…") or "Unban Personnel" ("Unbanning…") |
| role note | "Website access follows verified TBD Discord membership and role mappings.", "Current role: <role>" and "Change the member’s Discord roles to change their access." |
| ban dialog | "Ban personnel?", "A reason is required. This action is recorded on the roster.", "Ban reason" over the box "Reason (required)", "Cancel" and "Ban personnel" ("Banning…"), disabled while the reason is blank |
| warning dialog | "Issue warning?", "A reason is required. The warning count on this dossier updates after a successful POST.", "Warning reason" over the box "Reason (required)", "Cancel" and "Issue warning" ("Issuing…"), disabled while the reason is blank |
| toasts | "Personnel banned", "Personnel unbanned", "Warning issued", "Discord roles resynced (N user(s) updated)"; a refusal shows the API's sentence, else "Ban failed", "Unban failed", "Warning failed" or "Role sync failed"; "Role sync returned an unexpected response" |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `api_post`, `api_post_ok`, `api_delete`,
  `api_error_message`, `Paginated`, `AdminUserRow`), `crate::v2::core::auth` (`AuthStore`) and
  `crate::v2::core::ui` (`AdminGate`, `Dialog`, `MaterialIcon`, `cn`, the toast queue); over HTTP,
  the roster, ban, warning and role resync routes of the
  [administration](/documentation_v2/glossary/a_to_f.md#administration) domain.
- Used by: the `/admin/personnel` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Personnel Roster" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `personnel_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the page's sources for its
  tests; the DOM oracle's `personnel` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the role note offers no website override, with no picker, no input and no role request
  (`role_ui_explains_discord_authority_without_website_override`); a ban or a warning is never sent
  without a trimmed reason (`ok_with_blank_is_refused_before_any_request`,
  `whitespace_only_is_refused_too`, `a_real_reason_is_sent_trimmed`); the ban, warning and resync
  paths are held against the API's route tables
  (`admin_ban_and_warnings_paths_match_live_api_routes`,
  `admin_roles_sync_path_matches_live_api_route`); a banned member is offered an unban
  (`unban_control_deletes_ban_when_banned`), all in `tests/personnel.rs`.

## Related documentation

- [Personnel roster page](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
- [Administration domain](/apps/website/api_v2/src/administration/README.md) — the roster, ban,
  warning and role resync routes.
