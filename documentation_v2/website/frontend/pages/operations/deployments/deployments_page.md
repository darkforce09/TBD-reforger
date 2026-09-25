**Status:** live

# Deployments page

The `/deployments` page in the [operations](/documentation_v2/glossary.md#operations) section,
labelled "My Deployments": a signed-in member's own
[service record](/documentation_v2/glossary.md#service-record), with the missions they are
signed up for next, the matches they have played, the leave of absence they have filed and, for an
administrator, the queue of leave requests to decide.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/operations/deployments/`](/apps/website/frontend/src/v2/pages/operations/deployments/):
  `page.rs` holds the route component `DeploymentsPage`, the fetch and the two-column layout;
  `active_orders.rs` the banner of upcoming deployments; `service_record.rs` the combat history
  table; `leave_of_absence.rs` the leave form and the viewer's requests;
  `leave_review_queue.rs` the administrator's queue. The folder's
  [README](/apps/website/frontend/src/v2/pages/operations/deployments/README.md) describes each
  file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/operations/deployments/README.md#routes). The
  sidebar lists the page as "My Deployments" in the "Operations" section.
- Related: the [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
  and the [ORBAT selection page](/documentation_v2/website/frontend/pages/operations/orbat_selection/orbat_selection_page.md),
  which the banner links to; the [leaderboards page](/documentation_v2/website/frontend/pages/operations/leaderboards/leaderboards_page.md),
  which shows the combat figures this page leaves out; the
  [API](/documentation_v2/glossary.md#api)'s
  [operations domain](/apps/website/api_v2/src/operations/README.md), which serves the record and
  the leave requests.

## Behaviour

### The record

1. The page body sits in `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows
   the session states of the README's
   [States](/apps/website/frontend/src/v2/pages/operations/deployments/README.md#states) until the
   viewer is signed in.
2. The signed-in half reads the viewer's name and [role](/documentation_v2/glossary.md#role)
   from the session through a memo, so a profile poll that changes neither does not rebuild the
   page and discard an unsent leave form. It fetches the record once and shows "Loading…", then
   "Failed to load data." or the record.
3. The left column shows the viewer's name, their role, "Total Deployments" with the count of
   matches they have played, and "Personal Telemetry" reading "No telemetry recorded".
4. The right column opens with "Active Orders". With no upcoming
   [deployment](/documentation_v2/glossary.md#deployment) it reads "No Active Orders" and
   "Stand by for deployment tasking."; otherwise the banner shows the soonest one: its event's
   name (or "Untitled Operation"), a "Reservation: <state>" badge, an "Attendance: <state>" badge
   when there is one, the local start time, "T-MINUS " and the time left as one rounded unit,
   the terrain, and "Assigned slot: " with whichever of faction, squad and role the viewer's seat
   has.
5. The banner links to "Modify Assignment" (the
   [ORBAT selection page](/documentation_v2/website/frontend/pages/operations/orbat_selection/orbat_selection_page.md)
   for that mission) and "Operation Hub" (`/events/{id}`), each only when the ids it needs are on
   the wire. The other upcoming deployments follow under "Also Awaiting Deployment", each linking
   to its event's hub.
6. "Combat History" lists the matches the viewer played, with the columns "Date", "Operation",
   "Role Played", "Outcome" and "AAR": the outcome reads "Mission Success", "Failed", "Aborted",
   "Pending" or "Unknown", and the replay cell shows "View Replay" only for an `http(s)` URL and an
   em dash for anything else. With none, "No Service History Compiled".

### Leave of absence

1. The "Leave of Absence" panel holds "Starts on", "Ends on" and "Reason" (placeholder
   "Optional reason…") and a "Submit Leave of Absence" button that reads "Submitting…" while the
   request is out.
2. Before posting, the form checks the dates as the API does: both present
   ("starts_on and ends_on are required"), both a bare `YYYY-MM-DD` ("dates must be YYYY-MM-DD"),
   and the end on or after the start ("ends_on must be on or after starts_on"); a broken rule shows
   under the button and in a toast. The body carries the bare dates, and the reason only when one
   was typed.
3. A filed request toasts "Leave request submitted" and the panel fetches its list again. The
   list shows "Starts", "Ends", "Reason", "Status" and "Filed", or "No leave requests on file".
4. An administrator also sees the "LOA Review Queue", marked "Admin", with "Member", "Starts",
   "Ends", "Reason", "Status" and "Review". A pending request offers "Approve" and "Deny"
   ("LOA approved", "LOA denied"); a decided one shows who decided it. The queue fetches again
   after each decision.

### Known discrepancies

- The leave form calls the reason optional and leaves it out of the body when it is empty
  (`LeaveOfAbsencePanel` in
  `apps/website/frontend/src/v2/pages/operations/deployments/leave_of_absence.rs`, and
  `CreateLeaveInput` in `apps/website/frontend/src/v2/core/api/dto/events.rs`), but the API
  requires one: a body without it is refused with 400 "starts_on, ends_on and reason are
  required", and a blank one with 400 "reason is required" (`submit_leave` in
  `apps/website/api_v2/src/operations/handlers/leave_requests.rs`). A member who leaves the reason
  empty cannot file a request.
- The review queue shows the first 20 requests, pending first: the page sends no paging and has
  no pager (`AdminLeaveQueue` in
  `apps/website/frontend/src/v2/pages/operations/deployments/leave_review_queue.rs`), and the API
  pages by 20 by default (`list_all_leave`, same API file).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/operations/deployments/README.md#data)
lists each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/me/deployments` (`get_my_deployments` in
  `apps/website/api_v2/src/operations/handlers/member_service_record.rs`), for any signed-in
  member:
  - `upcoming`: the viewer's `registered` and `waitlisted` signups on missions that start after
    now, soonest first, in events and missions that are not deleted; each names the event (its
    `name_override`, else the mission's title), the mission's terrain and start, the reservation
    and attendance states, and the faction, squad and role of the seat assigned to the viewer, if
    any.
  - `service_history`: one row per match the viewer has player statistics in, with the match's
    date, operation, outcome and `aar_replay_url`, and the role the viewer played.
  - `total_operations`: the number of distinct matches the viewer played, kept on the user row
    (`recompute_user_stats_on_connection` in
    `apps/website/api_v2/src/command_center/services/user_stats.rs`).
  - `attendance_rate`, derived from the viewer's decided attendance only, and `kills`, `deaths`,
    `kd_ratio`, `command_games`, `command_wins` and `command_win_rate`, read from the same
    `leaderboard_totals` view the leaderboards use; a figure nobody measured is `null`, never `0`.
    The page shows none of them.
- `GET` and `POST /api/v1/me/leave-requests` (`list_my_leave` and `submit_leave` in
  `apps/website/api_v2/src/operations/handlers/leave_requests.rs`): the viewer's own requests, and
  filing one as `pending` with the trimmed reason (201).
- `GET /api/v1/admin/leave-requests` (`list_all_leave`, same file), administrators only: every
  request, pending first, 20 per page by default.
- `PATCH /api/v1/admin/leave-requests/{id}` (`review_leave`, same file), administrators only:
  sets `approved` or `denied` and records the reviewer; 400 "status must be approved or denied"
  for any other value, 404 "LOA not found" for an unknown id.

The page stores nothing in the browser.

## Design

- Two columns over the topographic backdrop: a 30% identity column and the main column with the
  "Active Orders" banner (drawn over an inline grid-and-reticle artwork), the history table, the
  leave panel and the review queue. The three tables share one column heading style
  (`ServiceHead`).
- Design target: the [service record blueprint](/documentation_v2/website/frontend/pages/operations/deployments/visual_references/service_record_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [My Deployments section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#3-my-deployments).
  The built page follows the blueprint's two columns and differs:
  - the identity column shows the name, the role and the deployment count, and "No telemetry
    recorded" where the blueprint shows a K/D ratio, a win rate, a favourite weapon and a
    favourite asset;
  - the banner countdown is one rounded unit ("T-MINUS 2 DAYS") rather than `24:00:00`, and the
    banner adds reservation and attendance badges and an "Operation Hub" link;
  - "Combat History" is a table without per-match K/D and with an AAR column, where the blueprint
    shows a timeline of cards with a K/D and a victory or defeat badge;
  - the leave panel and the review queue have no counterpart in the blueprint.
- The archived spec asked for a grid of deployment cards, each with a countdown and the assigned
  ORBAT slot, over a table of past operations with an AAR link; the built page shows the soonest
  deployment as a banner and the rest as a list beneath it.

## Open work

- [T-1031 — Add kills, deaths and K/D to the deployments page](/.ai/tickets/T-1031.toml) (idea,
  no plan): once telemetry flows, the identity column shows the kills, deaths and K/D the API
  already serves instead of "No telemetry recorded".
- [T-136 — 3D AAR / OCAP-style replay](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-136_plan.md)): a replay page for each match,
  which the history's `aar_replay_url` and its "View Replay" link lead to.

## Decisions

- No invented figures: the page shows only what the API measured, so the personal telemetry block
  reads "No telemetry recorded" rather than zeros
  (`no_fabricated_personal_telemetry_survives_in_this_module` and
  `personal_telemetry_empty_copy_is_pinned` in
  `apps/website/frontend/src/v2/pages/operations/deployments/tests/deployments.rs`).
- One fetch feeds the record: upcoming deployments and history arrive together, so nothing on the
  page can go stale against anything else on it; each leave panel owns its own fetch because it
  changes on its own.
- The replay cell emits a link only for an `http(s)` URL, checked again at render although the API
  checks it on write, because a stored `javascript:` or `data:` value would run in the link
  (`aar_cell_emits_an_href_only_for_http_urls`, same test file).
- The leave form's date rules match the API's, and only bare dates are posted
  (`loa_date_validation_matches_backend_rules` and `create_leave_body_is_bare_ymd_json`, same test
  file).
