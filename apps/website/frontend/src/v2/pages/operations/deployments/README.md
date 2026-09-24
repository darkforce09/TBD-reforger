# Deployments page

The `/deployments` page, titled "My Deployments": the viewer's own [service
record](/documentation_v2/glossary.md#service-record), with what they are slotted into next, the
[missions](/documentation_v2/glossary.md#mission) they have played, the leave they have filed, and
for an administrator the leave review queue.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/deployments/
├── active_orders.rs       the banner: the soonest deployment and the queue behind it
├── leave_of_absence.rs    the leave form, its date rules and the table of the viewer's own requests
├── leave_review_queue.rs  the administrator's queue of leave requests, with approve and deny
├── mod.rs                 the module tree; re-exports `DeploymentsPage`
├── page.rs                the route component: the deployments fetch and the two-column layout
├── service_record.rs      the combat history table and which stored replay strings become links
├── table_head.rs          `ServiceHead`: the column heading the three tables share
└── tests/                 unit tests for the replay link, leave rules and body, badges and telemetry
```

## How it works

`DeploymentsPage` renders inside `AuthGate`. The signed-in half reads the viewer's name and
[role](/documentation_v2/glossary.md#role) from the session through a memo, so a profile poll that
changes neither does not rebuild the page (a rebuild discards the leave form's unsent input), and
fetches the [deployments](/documentation_v2/glossary.md#deployment) payload once: both lists arrive
in it, so nothing on the page can go stale against anything else on it.

The left column shows the name, the role, "Total Deployments" and a "Personal Telemetry" block that
reads "No telemetry recorded". The right column holds the "Active Orders" banner, the "Combat
History" table, the leave panel and, for the `admin` role only, the "LOA Review Queue". The banner
shows the soonest upcoming deployment: its [event](/documentation_v2/glossary.md#event) name,
reservation and attendance badges, local start time, countdown, terrain and assigned
[slot](/documentation_v2/glossary.md#slot), a "Modify Assignment" link to the
[ORBAT](/documentation_v2/glossary.md#orbat) selection page and an "Operation Hub" link to the event
hub page, each rendered only when the ids it needs are on the wire; the others follow under "Also
Awaiting Deployment". The history table's replay cell emits a link only for an `http(s)` URL; any
other stored value, `javascript:` and `data:` included, shows the same em dash as an absent replay.

The leave form checks the dates before posting (both present, both a bare `YYYY-MM-DD`, the end on
or after the start) and posts only the bare dates. Each leave panel owns its own fetch and fetches
again after a submit or a decision; approve and deny show only on a pending request.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/deployments` | `DeploymentsPage` | route tier `none`; the data renders only for a signed-in viewer; the review queue only for `admin` | full-bleed inside the navigation frame; breadcrumb Operations › My Deployments |

## Data

- `GET /api/v1/me/deployments`: read as `Deployments`; the page renders `total_operations`,
  `upcoming` (`DeploymentUpcoming` rows) and the untyped `service_history` rows (`date`,
  `operation`, `role`, `outcome`, `aar_replay_url`). It renders none of the DTO's
  `attendance_rate`, `kills`, `deaths`, `kd_ratio`, `command_games`, `command_wins` and
  `command_win_rate`.
- `GET /api/v1/me/leave-requests`: the viewer's requests, read as `DataEnvelope<LeaveRequest>`.
- `POST /api/v1/me/leave-requests` with `CreateLeaveInput` (`starts_on`, `ends_on`, and `reason`
  when one is given), read as `LeaveRequest`.
- `GET /api/v1/admin/leave-requests`: the review queue, read as `Paginated<LeaveRequest>`.
- `PATCH /api/v1/admin/leave-requests/{id}` with `{"status":"approved"}` or `{"status":"denied"}`.
- The page reads the session (name and role) from the `AuthStore` context and stores nothing in the
  browser. Every fetch and submit runs in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load data." |
| no upcoming deployment | "No Active Orders" and "Stand by for deployment tasking." |
| no history | "No Service History Compiled" |
| history row | the date, operation, "Role Played", an outcome of "Mission Success", "Failed", "Aborted", "Pending" or "Unknown", and a "View Replay" link or an em dash |
| leave loading, failed, empty | "Loading leave requests…", "Failed to load leave requests.", "No leave requests on file" |
| leave submit | "Submitting…" on the button; a toast "Leave request submitted", or the date rule broken ("starts_on and ends_on are required", "dates must be YYYY-MM-DD", "ends_on must be on or after starts_on") under the button and in a toast |
| review queue loading, failed, empty | "Loading review queue…", "Failed to load LOA review queue.", "No leave requests in the queue" |
| review decided | a toast "LOA approved" or "LOA denied"; "Failed to approve LOA" or "Failed to deny LOA" on failure |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get`, `api_post` and `api_patch` client,
  `api_error_message`, `Deployments`, `DeploymentUpcoming`, `LeaveRequest`, `CreateLeaveInput`,
  `DataEnvelope`, `Paginated`), `crate::v2::core::auth` (`AuthStore`, `Role`, `url_guard`),
  `crate::v2::core::ui` (`AuthGate`, `MaterialIcon`, `badge_class`, the toasts) and
  `crate::v2::core::utils` (countdown and date formatting).
- Used by: the `/deployments` route in `apps/website/frontend/src/app_routes.rs`;
  `deployments_source` in `apps/website/frontend/src/v2/core/test_support/pins.rs` reads its source
  files.
- Rules: the replay cell emits an `href` only for an `http(s)` URL
  (`aar_cell_emits_an_href_only_for_http_urls` in `tests/deployments.rs`); the leave form's date
  rules match the [API](/documentation_v2/glossary.md#api)'s and the body carries bare dates
  (`loa_date_validation_matches_backend_rules`, `create_leave_body_is_bare_ymd_json`); no invented
  personal figures appear, and the empty telemetry text stays
  (`no_fabricated_personal_telemetry_survives_in_this_module`,
  `personal_telemetry_empty_copy_is_pinned`).

## Related documentation

- [Deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md)
  — the page's behaviour and design.
