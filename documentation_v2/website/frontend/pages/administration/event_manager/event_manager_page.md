**Status:** live

# Event manager page

The `/admin/events` page, titled "Operations Calendar": administrators schedule
[events](/documentation_v2/glossary.md#event), which the screen calls operations, on a month
calendar, attach [missions](/documentation_v2/glossary.md#mission) whose
[ORBAT](/documentation_v2/glossary.md#orbat) becomes each event mission's
[slots](/documentation_v2/glossary.md#slot), edit an operation's time, briefing, capacity, lifecycle
state and registration, delete it, and decide in an access sheet who may join, from which pools, and
why each participant is admitted.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/event_manager/`](/apps/website/frontend/src/v2/pages/administration/event_manager/):
  `page.rs` holds the route component `EventManagerPage`; `state.rs` the screen's state and its
  three fetches; `event_table.rs` the heading, the month grid and the day panel;
  `schedule_dialog.rs` and `edit_dialog.rs` the two forms; `mission_picker.rs` the mission
  staging list and the attached-missions roster; `confirm_dialogs.rs` the delete and detach
  confirmations; `lifecycle.rs` the six states and the moves between them; `dates.rs` the date
  arithmetic; `access/` the access sheet. The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/event_manager/README.md) describes
  each file.
- Entry: the `/admin/events` route renders `EventManagerPage`
  (`apps/website/frontend/src/app_routes.rs`); `apps/website/frontend/src/router.rs` declares it
  for the `admin` tier, padded rather than full-bleed, with the breadcrumb "Administration" ›
  "Event Manager", and the sidebar lists it as "Event Manager"
  (`apps/website/frontend/src/v2/pages/navigation/nav_config.rs`).
- Related: the [event manager](/documentation_v2/glossary.md#event-manager) glossary entry; the
  [event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
  and the [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md),
  where members see and join what this page schedules; the
  [API](/documentation_v2/glossary.md#api)'s
  [operations domain](/apps/website/api_v2/src/operations/README.md); the
  [event administration evidence](/documentation_v2/website/api_v2/verification_evidence/event_administration.md),
  [eligibility and allocation evidence](/documentation_v2/website/api_v2/verification_evidence/event_eligibility_allocation.md)
  and [reservation and attendance evidence](/documentation_v2/website/api_v2/verification_evidence/reservation_attendance.md).

## Behaviour

The page body sits in `AdminGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`): "Loading
session…" while the session restores, a sign-in prompt for a signed-out viewer, and "Admin access
required." below the `admin` [role](/documentation_v2/glossary.md#role).

### Calendar and day panel

1. The heading "Operations Calendar" carries the line "Schedule operations for any day. ORBATs
   generate from each attached mission." and a "Schedule Operation" button.
2. The month grid opens on the current month, with previous and next controls, weekday headers,
   today in bold and up to three marks on each day that has operations. A failed read of the
   operations leaves the grid without marks.
3. Selecting a day moves the day panel there and selects that day's first operation. The panel is
   headed by the local date and "Scheduled Operations" and lists each operation: its name
   ("Untitled Operation" when unnamed), "<local time> · N mission(s) · filled/total", its
   lifecycle state and "Open" or "Locked". An empty day reads "No operations scheduled. Schedule
   one.", where "Schedule one." opens the schedule form.
4. While an operation is selected the panel offers "Edit Selected Operation", "Access, Groups &
   Places" and "Delete Selected Operation".

### Scheduling an operation

1. "Schedule Operation" opens a dialog of that name for the selected day (its long date as a
   caption): a time (19:00 unless changed), a name ("Operation name (e.g. Twin Theaters)"), a
   "Missions" staging list ("No missions attached yet." until one is staged; "Attach Mission"
   offers the library, "No more missions in the library." once every one is staged), and
   Registration "Open" or "Locked".
2. "Publish Event" ("Publishing…" while it runs) needs a time ("Start time is required"). It
   creates the operation, then attaches each staged mission at the operation's start time, one
   request each. It toasts "Event published with N missions" (or "Event published" with none),
   resets the form and reads the calendar again; a failed create toasts "Failed to publish event".

### Editing an operation

1. "Edit Selected Operation" opens "Edit Operation": date, time, name, briefing ("Briefing
   (Markdown supported)"), banner image URL, "Max slots", the attached missions, the lifecycle
   "Status" and Registration "Open" or "Locked".
2. The attached missions ("Attached Missions") come from the operation's hub, read while the form
   is open: "Loading…", "Could not load attached missions." or "No missions attached.", else one
   row per mission with "<local time> · filled/total filled" and a detach control. "Attach
   Mission" ("Attaching…" while it runs) attaches another at the operation's start time and
   toasts "Attached <title>".
3. The Status picker offers only the moves the API accepts: `scheduled` to `open`, `locked`,
   `live` or `cancelled`; `open` to `locked`, `live` or `cancelled`; `locked` to `open`, `live` or
   `cancelled`; `live` to `open`, `locked`, `completed` or `cancelled`. Completed and cancelled
   operations are terminal, and the form says so: "Completed and cancelled operations are
   terminal — rerunning one is a new operation, not an edit.".
4. "Save Changes" ("Saving…" while it runs) sends only the fields that changed; with none it
   answers "No changes to save". A max-slots value that is not a whole number of 0 or more is
   refused in the form. A save toasts "Operation updated"; a refusal shows the API's sentence,
   such as "cannot move an event to open once its start time has passed — reschedule it in the
   same request to postpone it", else "Could not update operation".
5. Detaching a mission asks "Detach this mission?" and toasts "Mission detached", or the API's
   sentence, else "Could not detach mission".

### Deleting an operation

"Delete Selected Operation" asks "Delete this operation?" with the text "It leaves the schedule,
the dashboard and everyone's deployments, and no one can register on it. Nothing is erased — the
attached missions, their ORBATs and every registration are kept, so an administrator can still
restore it from the database." and a "Delete operation" button. It toasts "Operation deleted" or
"Failed to delete operation".

### Access sheet

1. "Access, Groups & Places" opens a side sheet over the calendar, "Access & Places", with the
   operation's name and "access revision N" and four tabs: Policies, Groups, Places, Participants.
   It reads "Loading access settings…" until the access view arrives, and nothing in it is
   editable before then.
2. Every change names the access revision the sheet was read at. The sheet then adopts the view
   the API answers with and reports what the change did: "Released: …", "Seated from the waiting
   list: …", or "No reservation was released or promoted.". When another administrator changed
   the settings first, the change is refused, the view reloads and the sheet says "Another
   administrator changed this operation's access settings after you loaded them, so your change
   was not applied. …".
3. Policies: the operation's policy first, then each mission's squads and slots, each stating
   where its policy comes from ("Own policy.", "Follows its squad's policy.", "Follows the
   operation's policy.", or "Own policy with no grants: admits nobody"). A slot inherits from its
   squad and a squad from the operation. "Give it its own policy" starts an own policy from the
   inherited one, "Edit own policy" edits it, and "Inherit again" removes it. The editor holds
   grants as alternatives, each with conditions that must all hold ("Any one grant admits an
   account; every condition inside a grant must hold."); a condition is any signed-in account, a
   verified TBD member, a holder of a Discord role (guild id and role id), a member of an event
   group, or one named account found through the member search. Saving a policy with no grants
   is allowed, and the editor warns first: "No grants: saved like this, the policy admits nobody.
   That closes every seat it decides — it does not inherit.". Each mission also offers "Promote
   from waiting list".
4. Groups: a managed roster ("Managed roster: administrators add and remove members"), listing who
   added each member and when, with a remove control and a member search to add; or a partner
   guild, with its guild id and the roles a member must hold ("Membership follows bot-verified
   Discord observations of the partner guild; there is no roster to edit."). Groups are created,
   renamed, switched between the two sources and deleted; the API refuses to delete a group some
   policy still names.
5. Places: the member, guest and open pools, each with a limit or "Uncapped" and an "Opens (UTC)"
   time, saved together with "Save pools". The sheet explains the rule: "Members draw from the
   member pool and everyone else from the guest pool; either overflows to the open pool once it
   opens. Zero closes a pool; uncapped leaves only the operation-wide limit. A limit cannot go
   below the places the pool already holds."
6. Participants: each participant's availability, TBD membership, place and pool, every
   reservation with the policy that decides it and the grants that admit it (numbered from 1),
   whether current and last-verified facts admit it, and the Discord guild observations and roster
   entries behind those facts; "Nobody holds a place, a seat or a waiting entry yet." when empty.

### Known discrepancies

- The detach confirmation says "The mission's ORBAT slots and every registration on it are
  deleted. The mission itself stays in the library. This cannot be undone.". The API deletes
  nothing: it hides the attachment, withdraws its registrations and keeps its ORBAT with the
  signup and attendance history, and attaching the same mission again restores the attachment
  with that ORBAT and history (`remove_event_mission` and `add_event_mission` in
  `apps/website/api_v2/src/operations/handlers/event_mission_attachment.rs`).
- The delete confirmation says every registration is kept. The API withdraws every reservation
  of the operation (reason `event_deleted`) and keeps the signup and attendance history
  (`delete_event` in `apps/website/api_v2/src/operations/handlers/event_create_update.rs`).
- The calendar asks for every operation without a limit (`GET /api/v1/events?scope=all`), so the
  API answers its default page: the 20 earliest operations ever scheduled. Later operations never
  reach the grid once more than 20 exist, and a failed read shows as an empty month
  (`state.rs` in the code folder).
- The schedule form ignores a failed mission attach: "Event published with N missions" counts the
  staged missions, not the attached ones (`schedule_dialog.rs`).
- Neither form sets an operation's server or modpack, which the API accepts; the server control
  page offers a deployment's event missions only from operations scheduled on that server.

## Data

The page README lists no calls, so the DTOs are named here. Server-side, in
`apps/website/api_v2/src/operations/handlers/`:

- `GET /api/v1/events?scope=all` (`list_events` in `event_listing.rs`): read as
  `Paginated<EventListItem>`. The `all` scope lists every operation the viewer may see, in start
  order, 20 per page unless `limit` asks for up to 100; each item adds `mission_count`,
  `registered`, `filled`, `total_slots` and `percent`.
- `GET /api/v1/missions?scope=global` (`list_missions` in
  `apps/website/api_v2/src/missions/handlers/mission_library.rs`): read as
  `Paginated<MissionCard>`, the picker's library: the live missions and the administrator's own
  missions that are not archived, most recently updated first, 20 of them since the page sends no
  `limit`.
- `GET /api/v1/events/{id}` (`get_event` in `event_hub.rs`): the `EventHub` behind the edit form's
  attached missions, and the access sheet's missions.
- `POST /api/v1/events` (`create_event` in `event_create_update.rs`) with `start_time`,
  `registration_locked` and an optional `name_override`: the API requires a start time, a
  `max_slots` of 0 to 256, a pre-start status (`scheduled`, `open` or `locked`), an absolute
  banner URL and a name that is not blank; it also accepts `server_id` and `modpack_id`. It
  records `event.created`.
- `PATCH /api/v1/events/{id}` (`update_event`) with only the changed fields: the API checks the
  capacity and the lifecycle move ("cannot move an event from X to Y"), refuses a move into a
  pre-start state (`scheduled`, `open` or `locked`) once the start time has passed unless the same
  request moves the start into the future, shifts every attached mission's start time by the same
  amount when the start moves, withdraws every reservation when the operation is cancelled
  (reason `event_cancelled`), then seats waiting participants the change makes room for and
  records `event.updated`.
- `DELETE /api/v1/events/{id}` (`delete_event`): withdraws every reservation, marks the operation
  deleted and records `event.deleted`; the row stays, so it can be restored in the database.
- `POST /api/v1/events/{id}/missions` with `{mission_id, start_time}` (`add_event_mission` in
  `event_mission_attachment.rs`): copies the mission's ORBAT into the event mission's slots and
  records `event.mission_attached`; it refuses a mission already attached and an archived
  mission (409), and restores a detached attachment of the same mission instead of creating one
  (`event.mission_restored`).
- `DELETE /api/v1/events/{id}/missions/{emid}` (`remove_event_mission`): withdraws the
  attachment's registrations (reason `mission_removed`), hides the attachment, seats waiting
  participants on the remaining missions and records `event.mission_removed`.
- The access sheet (`event_access_administration.rs`, `event_group_administration.rs`,
  `waitlist_promotion.rs`, `orbat_view.rs`):
  - reads `GET /api/v1/events/{id}/access` (`EventAccessAdministration`),
    `GET /api/v1/events/{id}/access/participants` (`ParticipantAccessExplanation` rows) and
    `GET /api/v1/event-missions/{emid}/orbat` for each mission;
  - changes policies with `PUT /api/v1/events/{id}/access-policy`, and `PUT` or `DELETE` on
    `/api/v1/event-missions/{emid}/squads/{faction}/{squad}/access-policy` and
    `/api/v1/event-missions/{emid}/slots/{slotId}/access-policy`; groups with
    `POST /api/v1/events/{id}/groups`, `PATCH` or `DELETE` on `/api/v1/events/{id}/groups/{groupId}`
    and `PUT` or `DELETE` on `/api/v1/events/{id}/groups/{groupId}/members/{discordId}`; the three
    pools together with `PUT /api/v1/events/{id}/reservation-quotas`; members are found with
    `GET /api/v1/members?q=`;
  - every change carries the access revision (in the body, or as `?expected_access_revision=` on a
    `DELETE`) and answers an `AccessChangeOutcome`: the new view with the registrations it
    released and promoted. The API locks the operation, checks the caller's authority again,
    refuses a stale revision with 409 `ACCESS_REVISION_CONFLICT`, re-evaluates the reservations
    (releasing only those whose ineligibility is confirmed), seats waiting participants and
    records the change. A policy holds at most 32 grants of 1 to 16 conditions, each id 1 to 128
    bytes;
  - `POST /api/v1/event-missions/{emid}/waitlist/promote` takes no revision: it seats the earliest
    eligible waiting participants in queue order, refuses with 409 `EVENT_FULL` when nobody can be
    seated (the sheet says "Nobody waiting on <mission> could be seated: …"), and admits leaders
    as well as administrators.

## Design

- A padded page, not full-bleed: the heading row, then on wide screens a 12-column grid with the
  month grid in eight columns and the day panel in four. The forms are frosted dialogs over the
  calendar, and the access sheet slides over it from the side.
- The access sheet as built:

```text
ACCESS SHEET (side sheet over the calendar)
+-----------------------------------------------------------------------+
| Access & Places — Op Resurrection · access revision 5          [ x ]  |
| <last change: what it released and promoted, or why it was refused>   |
| [ Policies ] [ Groups ] [ Places ] [ Participants ]                   |
+-----------------------------------------------------------------------+
| Policies:  Operation policy — Admits: Verified TBD member; or ...     |
|            Mission 1 [Promote from waiting list]                      |
|              BLUFOR / Alpha — Follows the operation's policy  [Give…] |
|                3. Rifleman — Own policy. Admits: ... [Edit] [Inherit] |
|            Editor: Grant 1 — all of [kind][fields] + condition        |
|                    + alternative grant   [Cancel] [Save policy]       |
| Groups:    roster / partner guild cards, provenance, members, search  |
| Places:    member / guest / open: [ ] uncapped [limit] opens (UTC)    |
| Participants: place, reservations, admitting grants, evidence         |
+-----------------------------------------------------------------------+
```

- Design target: the [event manager blueprint](/documentation_v2/website/frontend/pages/administration/event_manager/visual_references/event_manager_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Event Manager section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#8-event-manager).
  The built page differs from the blueprint:
  - the heading reads "Operations Calendar" with its own line, not "Event Manager" with
    "Schedule upcoming deployments and toggle registration locks.";
  - the blueprint's form beside the calendar (selected date, start time, one mission objective,
    registration toggle, "Publish Event" and "Delete Event") becomes the day panel, which lists
    the day's operations, and the schedule, edit and delete dialogs;
  - an operation carries several missions, a lifecycle state, a briefing, a banner and a slot
    ceiling, and the access sheet has no counterpart in the blueprint.

## Open work

- [T-1015 — Fix detach and delete confirm dialogs that misstate server effects](/.ai/tickets/T-1015.toml)
  (idea, no plan): the detach and delete confirmations say what the API does: detaching withdraws
  the attachment's registrations and keeps its ORBAT and history, and deleting withdraws every
  reservation.
- [T-1016 — Fix event manager calendar showing only the 20 earliest events](/.ai/tickets/T-1016.toml)
  (idea, no plan): the calendar receives every operation, not only the API's first page of 20,
  and a failed read no longer looks like an empty month.
- [T-1019 — Fix operation scheduling toasting failed mission attaches as success](/.ai/tickets/T-1019.toml)
  (idea, no plan): the schedule form checks each mission attach and reports a failed one instead
  of counting the staged missions.
- [T-1022 — Add website admin UI to manage game servers](/.ai/tickets/T-1022.toml) (idea, no
  plan): the event manager's forms set an operation's server, which the API already accepts, and
  a page creates, edits and deactivates servers.

## Decisions

- The Status picker mirrors the API's transition rules only so it never offers a refused move;
  the API owns the rules, and a rule the picker cannot check (reopening a started operation needs
  a reschedule into the future) comes back as the API's own sentence.
- The edit form sends only what changed: every field the API takes is optional and present means
  write, so sending the whole form would re-send the start time on a rename, and a start time in
  the body is what the API's pre-start guard measures. Blanking the briefing or the banner sends
  an empty string, which clears it.
- The edit form has a date field because reopening a started operation needs its start moved into
  the future in the same request; the schedule form has none, so an operation lands on the day
  the administrator was looking at.
- Access changes are guarded by the access revision instead of merged: a change prepared against
  an old view is refused and the view reloaded, rather than overwriting another administrator's
  change. An empty grant list admits nobody while removing an own policy inherits, and the sheet
  keeps the two visibly apart.
- Deleting and detaching hide rows and withdraw registrations rather than erase them, so an
  operation's history survives and a detached mission can be attached again with its ORBAT.
