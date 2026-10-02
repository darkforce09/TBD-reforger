# Personnel roster page

The `/admin/personnel` page, [personnel](/documentation/glossary/n_to_z.md#personnel), titled
"Personnel Roster": administrators search the member roster and page through it, open one member's
dossier beside it, ban, unban or warn a member, and resync every member's
[role](/documentation/glossary/n_to_z.md#role) from Discord. A role follows the member's
Discord roles, so the dossier explains it and never sets it.

## Contents

```text
apps/frontend/src/v2/pages/administration/personnel/
├── dossier.rs        one member's profile, four readings and the action buttons
├── member_roster.rs  the sort and filter modes, the passes that apply them, and the roster table
├── mod.rs            the module tree; re-exports `PersonnelRosterPage`
├── page.rs           `PersonnelRosterPage`: the gate, the URL address, the fetch and the header
├── role_dialog.rs    the role note, and the ban and warning dialogs with their required reason
├── roster_pager.rs   the page arithmetic of a served page and the pager bar under the table
├── roster_query.rs   the roster address: URL parse and format, request path and page sizes
└── tests/            unit tests for the routes, reasons, roster passes, address and pager
```

## How it works

`PersonnelRosterPage` renders `PersonnelInner` inside `AdminGate`. The inner component keeps the
roster's address in the URL: it reads `page`, `per_page` and `q` from the query string
(`use_query_map`) into a `RosterQuery`, and every control writes a new address by navigating in
place (`use_navigate` with `replace: true`, so paging and typing add no history entries). A
missing, non-numeric or out-of-range `page` reads as 1, and a `per_page` other than 10, 20, 50 or
100 reads as 20. The search text stays in the URL exactly as typed; the request sends it trimmed,
and not at all when it is blank. A new search or a new page size starts again at page 1.

One roster fetch is keyed on the address's request path (`RosterQuery::api_path`), so every
keystroke and every page move fetches again. The pager under the table (`roster_pager`) shows the
member count, "Previous", "Page n of m", "Next" and a per-page select of 10, 20, 50 or 100; it
reads the page, page size and total the API served (`PagerPosition`). Previous is disabled on the
first page, next on the last, and both until a page has been served. A page served past the end
(a hand-edited URL, or a roster that shrank) sends the address back to page 1 of the same search.

The sort and filter controls cycle through their modes and rearrange the rows of the loaded page
in the browser (`apply_roster_filter`, then `apply_roster_sort`), never fetching again; a caption
under the search box and each control's tooltip say they act on this page only. Every order
breaks ties on a second key, so rows that compare equal keep their places; the role order
compares the role's wire value.

Both panes read the same fetched page, so the dossier never shows a member the table no longer
lists. A row click sets `selected_id`, and `dossier` builds the pane from that row, seeding `role`,
`banned` and `warnings` signals so a ban, an unban or a warning shows at once; each also fetches
the roster again. The ban and warning dialogs need a reason: `classify_ban_reason` trims it, and
the confirm button stays disabled while it is blank (`reason_confirm_enabled`). An unban acts at
once, without a dialog. "Edit Roles" opens a note that sends nothing: the website sets no role, so
the page has no role picker and never calls the role route. "Sync Roles" posts the resync and
reports the `updated` count it answers; an answer without the count is reported as unexpected,
never as a completed sync. The dossier and its dialogs work on the loaded page: a selected member
on another page leaves the dossier pane empty until their page is back. Every
[API](/documentation/glossary/a_to_f.md#api) request runs in the browser build only; a native
build renders the failure branch.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/personnel` | `PersonnelRosterPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` role only | full-bleed inside the navigation frame; breadcrumb Administration / Personnel Roster; sidebar entry "Personnel Roster"; query `?page=<n>&per_page=<10\|20\|50\|100>&q=<text>`, written by the page, every part optional when arriving |

## Data

- `GET /api/v1/admin/users?page=<n>&per_page=<size>`, with `&q=<text>` for a search: read as
  `PersonnelPage` (`items`, `page`, `per_page`, `total`); the pager reads `page`, `per_page` and
  `total`, and the table and the dossier read each item's `discord_id`, `username`,
  `discord_handle`, `arma_id`, `arma_character`, `role`, `is_banned`, `warnings` and
  `total_deployments`.
- `POST /api/v1/admin/users/{discordId}/ban` with `{"reason": <text>}`, and `DELETE` on the same
  path to unban; the page reads only whether each succeeded.
- `POST /api/v1/admin/users/{discordId}/warnings` with `{"reason": <text>}`; likewise.
- `POST /api/v1/admin/roles/sync` with `{}`: the page reads the answer's `updated` count.
- The page reads the `AuthStore` context, the router's location and the toast queue; its only
  browser state is the address in the URL.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| header | "Personnel Roster", "Sync Roles" ("Syncing…" while it runs), the sort control ("Sort: Name", "Sort: Warnings", "Sort: Role", "Sort: Banned"; tooltip "Sorts the members on this page"), the filter control ("Filter: All", "Filter: Active", "Filter: Banned"; tooltip "Filters the members on this page"), the search field "Search Discord ID or Arma Name…" and the caption "Search covers the whole roster; sort and filter act on this page only." |
| roster loading | "Loading…" |
| roster failed | "Failed to load data." |
| no member | "No users found." (also on a page served past the end, until the page moves back to page 1) |
| roster | the columns "User", "Arma Character", "Rank", "Warnings" and "Status"; per member an initials badge with the Discord handle, else the username; the character, else the Arma id, else "Unlinked"; the role in capitals; the warning count, yellow above zero; and "Active" or "Banned" |
| pager | "N members" ("1 member"), "Previous", "Page n of m", "Next" and "Per page" with a select of 10, 20, 50 and 100 (accessible name "Members per page"); previous is disabled on page 1, next on the last page, and before the first answer both are disabled and the count and position are empty |
| no dossier | "Select personnel to view dossier" |
| dossier | the initials badge, the name, the Discord id and the Arma identity, or "Unlinked Arma identity"; the readings "Deployments", "Current Rank", "Warnings" and "Status"; "Edit Roles", "Issue Warning" ("Issuing…"), and "Ban Personnel" ("Banning…") or "Unban Personnel" ("Unbanning…") |
| role note | "Website access follows verified TBD Discord membership and role mappings.", "Current role: <role>" and "Change the member’s Discord roles to change their access." |
| ban dialog | "Ban personnel?", "A reason is required. This action is recorded on the roster.", "Ban reason" over the box "Reason (required)", "Cancel" and "Ban personnel" ("Banning…"), disabled while the reason is blank |
| warning dialog | "Issue warning?", "A reason is required. The warning count on this dossier updates after a successful POST.", "Warning reason" over the box "Reason (required)", "Cancel" and "Issue warning" ("Issuing…"), disabled while the reason is blank |
| toasts | "Personnel banned", "Personnel unbanned", "Warning issued", "Discord roles resynced (N user(s) updated)"; a refusal shows the API's sentence, else "Ban failed", "Unban failed", "Warning failed" or "Role sync failed"; "Role sync returned an unexpected response" |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `api_post`, `api_post_ok`, `api_delete`,
  `api_error_message`, `dto::administration::{PersonnelPage, AdminUserRow}`),
  `crate::v2::core::auth` (`AuthStore`), `crate::v2::core::ui` (`AdminGate`, `Dialog`,
  `MaterialIcon`, `Select`, `cn`, the toast queue), `leptos_router` (`use_query_map`,
  `use_location`, `use_navigate`) and the `url` crate's form encoding; over HTTP,
  the roster, ban, warning and role resync routes of the
  [administration](/documentation/glossary/a_to_f.md#administration) domain.
- Used by: the `/admin/personnel` route in `apps/frontend/src/app_routes.rs` and
  `apps/frontend/src/router.rs`; the sidebar's "Personnel Roster" link in
  `apps/frontend/src/v2/pages/navigation/nav_config.rs`; `personnel_source` in
  `apps/frontend/src/v2/core/test_support/pins.rs`, which joins the page's sources for its
  tests; the DOM oracle's `personnel` capture in
  `tools/developer_tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the role note offers no website override, with no picker, no input and no role request
  (`role_ui_explains_discord_authority_without_website_override`); a ban or a warning is never sent
  without a trimmed reason (`ok_with_blank_is_refused_before_any_request`,
  `whitespace_only_is_refused_too`, `a_real_reason_is_sent_trimmed`); the ban, warning and resync
  paths are held against the API's route tables
  (`admin_ban_and_warnings_paths_match_live_api_routes`,
  `admin_roles_sync_path_matches_live_api_route`); a banned member is offered an unban
  (`unban_control_deletes_ban_when_banned`), all in `tests/personnel.rs`; junk or out-of-range
  URL values fall back to page 1 and 20 per page, and a new search or page size starts at page 1
  (the `personnel_pagination_*` cases in `tests/roster_query.rs`); previous and next stop at the
  ends and a page past the end falls back to page 1 (the `personnel_pagination_*` cases in
  `tests/roster_pager.rs`).

## Related documentation

- [Personnel roster page](/documentation/apps/frontend/pages/administration/personnel/personnel_roster_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
- [Administration domain](/apps/api/src/administration/README.md) — the roster, ban,
  warning and role resync routes.
