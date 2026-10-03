**Status:** live

# Personnel roster page

The `/admin/personnel` page, titled "Personnel Roster": administrators search the member roster
and page through it, open one member's dossier beside it, ban or unban a member, issue a warning,
and resync every member's website [role](/documentation/glossary/n_to_z.md#role) from
Discord. A role follows the member's Discord roles, so the page explains it and never sets it.

## Where it lives

- Code: [`apps/frontend/src/pages/administration/personnel/`](/apps/frontend/src/pages/administration/personnel/):
  `page.rs` holds the route component `PersonnelRosterPage`, the address kept in the URL, the
  roster fetch, the search box, the sort and filter controls and the role resync;
  `roster_query.rs` the address (`page`, `per_page`, `q`): its URL parse and format and the
  request path; `roster_pager.rs` the page arithmetic and the pager bar; `member_roster.rs` the
  sort orders, the filters and the table; `dossier.rs` the member's profile, readings and action
  buttons; `role_dialog.rs` the role note and the ban and warning dialogs. The folder's
  [README](/apps/frontend/src/pages/administration/personnel/README.md) describes each
  file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/frontend/src/pages/administration/personnel/README.md#routes).
- Related: the [personnel](/documentation/glossary/n_to_z.md#personnel) glossary entry; the
  [API](/documentation/glossary/a_to_f.md#api)'s
  [administration domain](/apps/api/src/administration/README.md), which owns the
  roster, bans, warnings and the role resync; the membership grace extension, which the
  navigation frame's membership control sends, not this page.

## Behaviour

1. The page body sits in `AdminGate` (`apps/frontend/src/foundation/auth/gates.rs`), which
   shows the session and access states of the README's
   [States](/apps/frontend/src/pages/administration/personnel/README.md#states) in
   place of the page until a signed-in viewer holds the `admin` role.
2. The header holds the heading, the "Sync Roles" button, a sort control, a filter control, the
   search field and a caption saying that search covers the whole roster while sort and filter
   act on this page only.
3. The page shows one page of the roster at a time. Its address lives in the URL as `page`,
   `per_page` and `q` (`?page=2&per_page=50&q=vance`), so a paged or searched roster can be
   reloaded or shared. Every control rewrites the address in place, replacing the history entry
   rather than adding one. Arriving with no address shows page 1 at 20 per page with no search;
   a missing, non-numeric or out-of-range `page` reads as 1, and a `per_page` other than 10, 20,
   50 or 100 reads as 20.
4. The roster loads on arrival and again whenever the address changes: every keystroke in the
   search field, every page move and every page size. The search text stays in the URL as typed;
   the request sends it trimmed as `q`, and leaves `q` out when it is blank. A new search or a
   new page size starts again at page 1. The table's loading, failure and empty texts are in
   [States](/apps/frontend/src/pages/administration/personnel/README.md#states).
5. The pager under the table shows the member count ("6 members"), "Previous", the position
   ("Page 1 of 1"), "Next" and a "Per page" select of 10, 20, 50 and 100. It reads the page, page
   size and total the API served; a roster always fills at least one page. Previous is disabled
   on the first page and next on the last. A page served past the end, from a hand-edited URL or
   a roster that shrank, sends the address back to page 1 of the same search.
6. The sort control cycles four orders (name, warnings, role, banned first) and the filter
   control three subsets (all, active, banned). Both rearrange the rows of the loaded page in the
   browser and never refetch, so they order and narrow this page only; each control's tooltip
   says so too.
7. The table shows one row per member of the page, with the columns the README's
   [States](/apps/frontend/src/pages/administration/personnel/README.md#states) list.
8. Selecting a row opens that member's dossier: the profile, four readings and three buttons,
   "Edit Roles", "Issue Warning", and "Ban Personnel" or "Unban Personnel". The dossier reads the
   loaded page, so moving to a page without the selected member empties the dossier pane.
9. "Edit Roles" opens a note, not a form, saying that access follows the member's Discord roles.
   It sends nothing.
10. "Ban Personnel" opens a dialog whose confirm button stays disabled until the reason holds
    more than whitespace. A ban flips the dossier's status and refetches the page. "Unban
    Personnel" acts at once, without a dialog.
11. "Issue Warning" opens a dialog with the same reason rule. A warning adds one to the dossier's
    count and refetches the page.
12. "Sync Roles" resyncs every member, reports how many were updated, then refetches. An answer
    without the count is reported as unexpected; a refusal shows the server's sentence, as the
    ban, unban and warning errors do. Every toast is in the README's
    [States](/apps/frontend/src/pages/administration/personnel/README.md#states).

### Known discrepancies

- The search placeholder promises a Discord id search ("Search Discord ID or Arma Name…",
  `apps/frontend/src/pages/administration/personnel/page.rs`), but the API matches
  the text against the username, the Discord handle, the Arma character and the Arma id only
  (`list_users` in `apps/api/src/administration/handlers/personnel_roster.rs`); a
  Discord id finds nobody unless it also appears in one of those.

## Data

The README's [Data](/apps/frontend/src/pages/administration/personnel/README.md#data)
lists each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/admin/users?page=<n>&per_page=<size>&q=<text>` (`list_users` in
  `apps/api/src/administration/handlers/personnel_roster.rs`): answers
  `{items, page, per_page, total}` (`personnel-roster.schema.json`). The API orders by
  `lower(username)`, then `discord_id`, so every member sits on exactly one page; `page` defaults
  to 1 and `per_page` to 20, a `per_page` above 100 is served as 100, and a `page` or `per_page`
  below 1 or not a number answers 400, which the page's URL fallbacks never send. It trims `q` and
  matches it case-insensitively as described above; `total` counts every match, and a page past
  the end answers no items with the real total.
- `POST /api/v1/admin/users/{discordId}/ban` (`ban_user` in
  `apps/api/src/administration/handlers/disciplinary.rs`): answers `{banned: true}`.
  In one transaction the API marks the member banned with the reason, the banning administrator
  and the time, revokes their refresh tokens (so the ban takes hold when the current access token
  expires), queues a re-evaluation of their [event](/documentation/glossary/a_to_f.md#event)
  reservations and records `user.ban` at warning severity. A blank reason is refused with 400
  "reason is required"; an unknown member with 404.
- `DELETE /api/v1/admin/users/{discordId}/ban` (`unban_user`): answers `{banned: false}`, queues
  the same re-evaluation and records `user.unban`.
- `POST /api/v1/admin/users/{discordId}/warnings` (`issue_warning`): creates the
  warning (201) and records `user.warn` at warning severity; the same reason rule applies.
- `POST /api/v1/admin/roles/sync` (`resync_roles` in
  `apps/api/src/administration/handlers/role_management.rs`): reapplies the guild's
  role mappings to every member, moves members no longer in the guild to `guest`, records
  `roles.resync` and answers `{updated}`, the number of members it updated.
- `PATCH /api/v1/admin/users/{discordId}` (`update_user`, same file) is never called: it refuses
  every request with 409 "Website roles are derived from Discord; change the Discord role mapping
  or use a membership grace extension".

## Design

- A 70/30 split: the roster table on the left under the header, the dossier on the right.
- Initials badges stand in for avatars; the role reads as its wire value in capitals; warnings
  above zero turn yellow; the status shows as a success or error badge.
- Design target: the [personnel roster blueprint](/documentation/apps/frontend/pages/administration/personnel/visual_references/personnel_roster_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Personnel Roster section](/documentation/archive/go_and_react_era_design/platform_context_handoff.md#10-personnel-roster).
  The built page differs from the blueprint:
  - no subtitle under the heading;
  - "Sort by Warnings" and "Filter by Rank" are one sort control and one filter control that
    cycle, and the filter picks All, Active or Banned rather than a rank;
  - a "Sync Roles" button joins the header;
  - initials badges replace photos, and the dossier's readings sit in a grid of tiles rather
    than telemetry rows;
  - the dossier's buttons open a role note and the ban and warning dialogs;
  - the "Total Records" count becomes the member count of the pager under the table, beside
    "Previous", the page position, "Next" and the page size.

## Open work

- [T-1017 — Fix personnel search placeholder promising Discord ID search](/.ai/tickets/T-1017.toml)
  (idea, no plan): the placeholder and the search agree: either the placeholder stops promising a
  Discord id search, or the API matches the Discord id too.

## Decisions

- Roles follow Discord: the website never sets a role, so the dossier explains where a role comes
  from and the API refuses a role change; access changes by changing the member's Discord roles
  or the role mappings, and "Sync Roles" applies them at once.
- A ban and a warning each need a written reason: both are moderation records that keep their
  author, and the reason is what the record says.
- Sorting and filtering run on the loaded page: switching them costs no request, and the captions
  say they act on this page, since the roster route pages and searches but neither sorts nor
  filters.
- The address lives in the URL and is rewritten in place: a paged or searched roster survives a
  reload and can be shared, and paging or typing does not flood the history. The page sizes are
  the four the select offers, so the select always shows the size the table was fetched with.
