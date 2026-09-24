**Status:** live

# Personnel roster page

The `/admin/personnel` page, titled "Personnel Roster": administrators search the member roster,
open one member's dossier beside it, ban or unban a member, issue a warning, and resync every
member's website [role](/documentation_v2/glossary.md#role) from Discord. A role follows the
member's Discord roles, so the page explains it and never sets it.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/personnel/`](/apps/website/frontend/src/v2/pages/administration/personnel/):
  `page.rs` holds the route component `PersonnelRosterPage`, the roster fetch, the search box,
  the sort and filter controls and the role resync; `member_roster.rs` the sort orders, the
  filters and the table; `dossier.rs` the member's profile, readings and action buttons;
  `role_dialog.rs` the role note and the ban and warning dialogs. The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/personnel/README.md) describes each
  file.
- Entry: the `/admin/personnel` route renders `PersonnelRosterPage`
  (`apps/website/frontend/src/app_routes.rs`); `apps/website/frontend/src/router.rs` declares it
  for the `admin` tier, full-bleed, with the breadcrumb "Administration" › "Personnel Roster",
  and the sidebar lists it as "Personnel Roster"
  (`apps/website/frontend/src/v2/pages/navigation/nav_config.rs`).
- Related: the [personnel](/documentation_v2/glossary.md#personnel) glossary entry; the API's
  [administration domain](/apps/website/api_v2/src/administration/README.md), which owns the
  roster, bans, warnings and the role resync; the membership grace extension, which the
  navigation frame's membership control sends, not this page.

## Behaviour

1. The page body sits in `AdminGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`): "Loading
   session…" while the session restores, a sign-in prompt for a signed-out viewer, and "Admin
   access required." below the `admin` role.
2. The header holds the heading "Personnel Roster", the "Sync Roles" button, a sort control, a
   filter control and the search field "Search Discord ID or Arma Name…".
3. The roster loads on arrival and again on every keystroke in the search field, which sends its
   text as `q` whenever it is not empty. The table shows "Loading…" while a request runs,
   "Failed to load data." when it fails and "No users found." when nothing matches. The page
   never asks for a further page, so it shows the first 20 members the API returns, in name
   order.
4. The sort control cycles "Sort: Name", "Sort: Warnings", "Sort: Role" and "Sort: Banned"; the
   filter control cycles "Filter: All", "Filter: Active" and "Filter: Banned". Both rearrange the
   loaded rows in the browser and never refetch.
5. The table's columns are User (an initials badge and the Discord handle, else the username),
   Arma Character (the character, else the Arma id, else "Unlinked"), Rank (the role's wire value
   in capitals), Warnings (yellow above zero) and Status ("Active" or "Banned").
6. Selecting a row opens the dossier; until then it reads "Select personnel to view dossier". The
   dossier shows the initials badge, the name, the Discord id and the Arma identity ("Unlinked
   Arma identity" when there is none), four readings (Deployments, Current Rank, Warnings,
   Status) and three buttons: "Edit Roles", "Issue Warning", and "Ban Personnel" or "Unban
   Personnel".
7. "Edit Roles" opens a note, not a form: "Website access follows verified TBD Discord membership
   and role mappings.", "Current role: …" and "Change the member’s Discord roles to change their
   access.". It sends nothing.
8. "Ban Personnel" opens "Ban personnel?" ("A reason is required. This action is recorded on the
   roster."); its confirm button stays disabled until the reason holds more than whitespace. A
   ban answers "Personnel banned", flips the dossier's status and refetches the roster. "Unban
   Personnel" acts at once, without a dialog, and answers "Personnel unbanned".
9. "Issue Warning" opens "Issue warning?" ("A reason is required. The warning count on this
   dossier updates after a successful POST."), with the same reason rule. A warning answers
   "Warning issued", adds one to the dossier's count and refetches the roster.
10. "Sync Roles" (reading "Syncing…" while it runs) resyncs every member and answers "Discord
    roles resynced (N user(s) updated)", then refetches. An answer without the count reads "Role
    sync returned an unexpected response"; a refusal shows the server's sentence, else "Role sync
    failed". The ban, unban and warning errors likewise show the server's sentence.

### Known discrepancies

- The search placeholder promises a Discord id search ("Search Discord ID or Arma Name…",
  `apps/website/frontend/src/v2/pages/administration/personnel/page.rs`), but the API matches
  the text against the username, the Discord handle, the Arma character and the Arma id only
  (`list_users` in `apps/website/api_v2/src/administration/handlers/personnel_roster.rs`); a
  Discord id finds nobody unless it also appears in one of those.

## Data

The page README lists no calls, so the DTOs are named here. Server-side:

- `GET /api/v1/admin/users` and `?q=<text>` (`list_users` in
  `apps/website/api_v2/src/administration/handlers/personnel_roster.rs`): read as
  `Paginated<AdminUserRow>` (`data`, `total`, `limit`, `offset`); a row carries `discord_id`,
  `username`, `discord_handle`, `arma_id`, `arma_character`, `role`, `is_banned`, `warnings` and
  `total_deployments` (`apps/website/frontend/src/v2/core/api/dto/auth.rs`). The API orders by
  username, pages by `limit` (20 by default, at most 100) and `offset`, trims `q` and matches it
  case-insensitively as described above; `total` counts every match.
- `POST /api/v1/admin/users/{discordId}/ban` with `{reason}` (`ban_user` in
  `apps/website/api_v2/src/administration/handlers/disciplinary.rs`): answers `{banned: true}`.
  In one transaction the API marks the member banned with the reason, the banning administrator
  and the time, revokes their refresh tokens (so the ban takes hold when the current access token
  expires), queues a re-evaluation of their [event](/documentation_v2/glossary.md#event)
  reservations and records `user.ban` at warning severity. A blank reason is refused with 400
  "reason is required"; an unknown member with 404.
- `DELETE /api/v1/admin/users/{discordId}/ban` (`unban_user`): answers `{banned: false}`, queues
  the same re-evaluation and records `user.unban`.
- `POST /api/v1/admin/users/{discordId}/warnings` with `{reason}` (`issue_warning`): creates the
  warning (201) and records `user.warn` at warning severity; the same reason rule applies.
- `POST /api/v1/admin/roles/sync` with `{}` (`resync_roles` in
  `apps/website/api_v2/src/administration/handlers/role_management.rs`): reapplies the guild's
  role mappings to every member, moves members no longer in the guild to `guest`, records
  `roles.resync` and answers `{updated}`, the number of members it updated.
- `PATCH /api/v1/admin/users/{discordId}` (`update_user`, same file) is never called: it refuses
  every request with 409 "Website roles are derived from Discord; change the Discord role mapping
  or use a membership grace extension".

## Design

- A 70/30 split: the roster table on the left under the header, the dossier on the right.
- Initials badges stand in for avatars; the role reads as its wire value in capitals; warnings
  above zero turn yellow; the status shows as a success or error badge.
- Design target: the [personnel roster blueprint](/documentation_v2/website/frontend/pages/administration/personnel/visual_references/personnel_roster_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Personnel Roster section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#10-personnel-roster).
  The built page differs from the blueprint:
  - no subtitle under the heading;
  - "Sort by Warnings" and "Filter by Rank" are one sort control and one filter control that
    cycle, and the filter picks All, Active or Banned rather than a rank;
  - a "Sync Roles" button joins the header;
  - initials badges replace photos, and the dossier's readings sit in a grid of tiles rather
    than telemetry rows;
  - the dossier's buttons open a role note and the ban and warning dialogs;
  - no record count under the table.

## Open work

- [T-940.7 — Users list: server page metadata and admin pager](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_7_plan.md)): the roster gains a pager, so
  an administrator can reach every member instead of the first 20; the API half (`total`,
  `limit`, `offset`) already answers.

## Decisions

- Roles follow Discord: the website never sets a role, so the dossier explains where a role comes
  from and the API refuses a role change; access changes by changing the member's Discord roles
  or the role mappings, and "Sync Roles" applies them at once.
- A ban and a warning each need a written reason: both are moderation records that keep their
  author, and the reason is what the record says.
- Sorting and filtering run on the loaded rows: switching them costs no request.
