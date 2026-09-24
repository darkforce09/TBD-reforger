# Event manager page

The `/admin/events` page, the [event manager](/documentation_v2/glossary.md#event-manager), headed
"Operations Calendar": administrators schedule [events](/documentation_v2/glossary.md#event), which
the screen calls operations, on a month grid, attach
[missions](/documentation_v2/glossary.md#mission) to them, edit and delete them, and open the access
sheet that decides who may join each one.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/event_manager/
├── access/              the access sheet: policies, groups, places and participant evidence
├── confirm_dialogs.rs   the delete and detach confirmations and the requests they guard
├── dates.rs             local day keys, the form field values and the instants they combine to
├── edit_dialog.rs       the "Edit Operation" form and the save that sends only what changed
├── event_table.rs       the heading, the month grid and the day panel with its controls
├── lifecycle.rs         the six lifecycle states, the moves the API accepts and the delete copy
├── mission_picker.rs    the schedule form's staged missions and the edit form's attached missions
├── mod.rs               the module tree; re-exports `EventManagerPage`
├── page.rs              `EventManagerPage`: the gate, then the panels and the sheet in stacking order
├── schedule_dialog.rs   the "Schedule Operation" form and its two-step publish
├── state.rs             `Manager`: calendar position, form fields, busy flags and the three fetches
└── tests/               unit tests pinning the edit form's wiring and the delete confirmation's copy
```

## How it works

`EventManagerPage` renders `EventManagerInner` inside `AdminGate`. The inner component builds one
copyable `Manager` handle, which every panel takes instead of a parameter list, so the calendar's
position, both forms' fields and the three fetches have one source. Its fetches are the event list,
the mission library the attach pickers offer, and the hub of the event the edit form is open on;
that last one is keyed on the form being open, so clicking through a day costs no request, and it
answers with the event id it belongs to, so the attached missions show only under the event they
were fetched for. `dates.rs` groups events by the local calendar day of their start, so the grid,
the day panel and the forms agree on which day an event is on; only the value sent is UTC.

| Surface | Opened by | What it sends |
|---|---|---|
| schedule form | "Schedule Operation", or "Schedule one." on an empty day | the create, then one attach per staged mission at the event's start time, a failed attach going unreported; then the list refetches |
| edit form | "Edit Selected Operation" | a `PATCH` of the changed fields only, diffed against the list row the form was seeded from; attaches and detaches |
| delete confirmation | "Delete Selected Operation" | the delete; then the list refetches |
| detach confirmation | a mission's detach control in the edit form | the detach; then the list and the attached missions refetch |
| access sheet | "Access, Groups & Places" | the changes in [access/](/apps/website/frontend/src/v2/pages/administration/event_manager/access/README.md) |

The edit form compares start times as instants, sends an empty string to clear the briefing or the
banner, and sends nothing when nothing changed. Its Status picker offers only the moves
`lifecycle::can_transition` allows, which mirror the API's rules; a rule the browser cannot check
comes back as the API's own sentence. Neither form sets an event's server or modpack, and neither
posts to Discord. The detach confirmation renders last because it shares a stacking level with the
edit form that opens it. Every request runs in the browser build only; a native build resolves each
fetch to nothing.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/events` | `EventManagerPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` role only | padded, not full-bleed, inside the navigation frame; breadcrumb Administration › Event Manager; sidebar entry "Event Manager" |

## Data

- `GET /api/v1/events?scope=all`: read as `Paginated<EventListItem>`; the grid and the day panel
  read `id`, `name_override`, `start_time`, `status`, `registration_locked`, `mission_count`,
  `filled` and `total_slots`, and the edit form seeds from `briefing`, `banner_image_url` and
  `max_slots` as well. The page sends no `limit`.
- `GET /api/v1/missions?scope=global`: read as `Paginated<MissionCard>`; both pickers offer its
  `id`, `title` and `terrain`, minus the missions already attached.
- `GET /api/v1/events/{id}`: read as `EventHub` while the edit form is open; its `missions`
  (`event_mission_id`, `mission_id`, `title`, `start_time`, `filled`, `total`) are the attached
  missions.
- `POST /api/v1/events`: sends `start_time`, `registration_locked` and, when a name is typed,
  `name_override`; the answer's `id` keys the attaches that follow.
- `POST /api/v1/events/{id}/missions`: sends `{mission_id, start_time}`, with the event's start time.
- `PATCH /api/v1/events/{id}`: sends the changed fields among `start_time`, `name_override`,
  `briefing`, `banner_image_url`, `max_slots`, `registration_locked` and `status`.
- `DELETE /api/v1/events/{id}` and `DELETE /api/v1/events/{id}/missions/{emid}`: delete the event,
  detach one mission.
- The access sheet:
  - `GET /api/v1/events/{id}/access`: read as `EventAccessAdministration`.
  - `GET /api/v1/events/{id}/access/participants`: read as `Vec<ParticipantAccessExplanation>`.
  - `GET /api/v1/events/{id}`, then `GET /api/v1/event-missions/{emid}/orbat` per mission: read as
    `EventHub` and `DataEnvelope<OrbatSquad>`.
  - `PUT /api/v1/events/{id}/access-policy`: sends `AccessPolicyChange`.
  - `PUT` and `DELETE /api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy`, and
    `PUT` and `DELETE /api/v1/event-missions/{emid}/slots/{slotId}/access-policy`: the `PUT` sends
    `AccessPolicyChange`.
  - `POST /api/v1/events/{id}/groups`: sends `EventGroupCreation`.
  - `PATCH` and `DELETE /api/v1/events/{id}/groups/{groupId}`: the `PATCH` sends `EventGroupChange`.
  - `PUT` and `DELETE /api/v1/events/{id}/groups/{groupId}/members/{discordId}`: the `PUT` sends
    `AccessRevisionPrecondition`.
  - `PUT /api/v1/events/{id}/reservation-quotas`: sends `ReservationQuotaChange`.
  - Each of these changes names the access revision, in the body or, on a `DELETE`, as
    `?expected_access_revision=`, and reads the answer as `AccessChangeOutcome`.
  - `GET /api/v1/members?q=<text>`: read as `DataEnvelope<Member>`, asked only for a non-blank query.
  - `POST /api/v1/event-missions/{emid}/waitlist/promote` with `{}`: read as `WaitlistPromotion`.
- The page reads the `AuthStore` context and the toast queue, and stores nothing in the browser.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| calendar | "Operations Calendar", "Schedule operations for any day. ORBATs generate from each attached mission." and "Schedule Operation"; the month grid with previous and next controls, today in bold, and up to three marks on each day with events |
| list loading or failed | the grid without marks, and every day reads as empty |
| empty day | the local date, "Scheduled Operations", "No operations scheduled. " and the link "Schedule one." |
| day with events | per event its name ("Untitled Operation" when unnamed), "<local time> · N mission(s) · filled/total", its lifecycle badge and "Open" or "Locked"; with one focused, "Edit Selected Operation", "Access, Groups & Places" and "Delete Selected Operation" |
| schedule form | "Schedule Operation" over the long date; the time (19:00 at first), "Operation name (e.g. Twin Theaters)", "Missions" with "No missions attached yet." until one is staged, "Attach Mission" and "No more missions in the library."; "Registration" "Open" or "Locked"; "Publish Event" ("Publishing…") |
| edit form | "Edit Operation": start date and time, name, "Briefing (Markdown supported)", "Banner image URL", "Max slots", "Attached Missions", "Status", "Registration" and "Save Changes" ("Saving…"); a terminal event's picker is disabled under "Completed and cancelled operations are terminal — rerunning one is a new operation, not an edit." |
| attached missions | "Loading…", "Could not load attached missions.", "No missions attached.", or per mission "<local time> · filled/total filled" with a detach control; "Attach Mission" ("Attaching…") |
| delete confirmation | "Delete this operation?", "It leaves the schedule, the dashboard and everyone's deployments, and no one can register on it. Nothing is erased — the attached missions, their ORBATs and every registration are kept, so an administrator can still restore it from the database.", "Cancel" and "Delete operation" |
| detach confirmation | "Detach this mission?", "The mission's ORBAT slots and every registration on it are deleted. The mission itself stays in the library. This cannot be undone.", "Cancel" and "Detach mission" |
| calendar toasts | "Event published with N mission(s)", N counting the staged missions, or "Event published", "Failed to publish event"; "Operation updated", "No changes to save", the API's sentence or "Could not update operation"; "Attached <title>", the API's sentence or "Could not attach mission"; "Operation deleted", "Failed to delete operation"; "Mission detached", the API's sentence or "Could not detach mission" |
| form refusals | "Start time is required", "Start date is required", "Max slots must be a whole number of 0 or more" |
| access sheet loading | "Access & Places", the event's name and "Loading access settings…" |
| access sheet failed | the API's sentence, else "Could not load the access settings" |
| access sheet loaded | the name followed by " · access revision N", and the tabs "Policies", "Groups", "Places" and "Participants" |
| Policies tab | "Operation policy" with "Admits: …" or "Admits nobody: no account can see or join this operation except through a squad or slot policy." and "Edit"; "Loading missions…" or the missions failure ("Could not load the missions"); per mission "Promote from waiting list", and per squad and slot "Own policy. …", "Follows its squad's policy. …", "Follows the operation's policy. …" or "Own policy with no grants: admits nobody", with "Give it its own policy" or "Edit own policy", and "Inherit again" |
| policy editor | "Any one grant admits an account; every condition inside a grant must hold.", "Grant N — all of", the kinds "Verified TBD member", "Member of an event group", "Holds a Discord role", "One named account" and "Any signed-in account", "+ Add a condition", "Remove grant", "Add an alternative grant", "Cancel" and "Save policy"; with no grants, "No grants: saved like this, the policy admits nobody. That closes every seat it decides — it does not inherit." |
| Groups tab | "A group is a named set of accounts a policy can admit: a roster you maintain, or the verified members of a partner guild.", "New group" ("Close") and "Create group"; "This operation has no groups yet."; per group its source ("Managed roster: administrators add and remove members" or "Partner guild <id>: …"), "Created by <who>, <UTC time>", "Edit" ("Close editor") and "Delete", confirmed by "Delete this group? A group that any policy still names cannot be deleted until every policy drops it." ("Keep" or "Delete group"); a roster reads "Roster · N" with "Added by …" and "Remove" per member and "Search members by name" ("No matching members."); a partner guild reads "Membership follows bot-verified Discord observations of the partner guild; there is no roster to edit." |
| Places tab | "N of M operation places held — member a · guest b · open c", or "N places held; no operation-wide limit — …", ending " · N from before the pools" when some are; "Member places", "Guest places" and "Open places", each with "Uncapped", a limit and "Opens (UTC)"; "Members draw from the member pool and everyone else from the guest pool; either overflows to the open pool once it opens. Zero closes a pool; uncapped leaves only the operation-wide limit. A limit cannot go below the places the pool already holds." and "Save pools" |
| Participants tab | "Loading participants…", the failure ("Could not load the participants"), or "Nobody holds a place, a seat or a waiting entry yet."; else per participant "Verified TBD member" or "Not a verified TBD member" (with " · account unavailable (banned or removed)"), "Holds a <pool> place since <UTC time>" or "Holds no place", each reservation's mission, seat and deciding policy, "Admitted by grant N" or "No grant admits it now", whether current and last-verified facts admit it, and each guild observation and roster entry |
| change notice | the change's name ("Operation policy saved", "Group <name> created", "Reservation pools saved" and the like, also toasted), then "Released: …", "Seated from the waiting list: …" or "No reservation was released or promoted." |
| change refused | the API's sentence, else "The change was refused"; a stale revision reads "Another administrator changed this operation's access settings after you loaded them, so your change was not applied. The latest settings are shown now; apply your change again if it still stands." |
| promotion | "Seated from the waiting list of <mission>" or "Nobody waiting on <mission> could be seated"; a refusal with `EVENT_FULL` reads "Nobody waiting on <mission> could be seated: the operation or the pools its waiters draw from are full." |
| access form refusals | a refused group, policy or pool form toasts its reason, such as "A group name needs 1 to 128 characters", "Grant N needs 1 to 16 conditions" or "The <pool> pool needs an opening date and time (UTC)" |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `api_post`, `api_patch`, `api_delete`,
  `api_error_message`; `Paginated`, `EventListItem`, `EventHub`, `EventMissionDossier`,
  `MissionCard` and the access DTOs), `crate::v2::core::auth` (`AuthStore`), `crate::v2::core::ui`
  (`AdminGate`, `Dialog`, `MaterialIcon`, the badge classes, the toast queue) and
  `crate::v2::core::utils` (local date formatting); over HTTP, the events, event mission and access
  routes of the [operations](/documentation_v2/glossary.md#operations) domain and the mission
  library of the missions domain.
- Used by: the `/admin/events` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Event Manager" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `event_manager_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs`, which joins the page's sources for its
  tests; the DOM oracle's `eventmgr` capture in
  `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: the edit form sends only changed fields and clears the briefing and banner with an empty
  string (`edit_dialog_reattach_and_empty_string_clear_are_wired`); the delete confirmation never
  promises a permanent cascade (`delete_confirm_copy_matches_the_soft_delete_handler`), both in
  `tests/event_manager.rs`; the Status picker offers only moves `lifecycle::can_transition`
  allows, and the API stays the judge.

## Related documentation

- [Event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the event, attachment and
  access routes this page calls.
