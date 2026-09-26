**Status:** live

# Event hub page

The `/events/:id` page in the [operations](/documentation_v2/glossary/n_to_z.md#operations) section:
one [event](/documentation_v2/glossary/a_to_f.md#event)'s hub, with its start time, briefing and places,
a dossier for each attached [mission](/documentation_v2/glossary/g_to_m.md#mission), and the inline
[ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) selector through which a signed-in member takes a
[slot](/documentation_v2/glossary/n_to_z.md#slot), holds a place without a seat or joins a waiting list,
and through which a leader reserves and fills a squad.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/operations/event_detail/`](/apps/website/frontend/src/v2/pages/operations/event_detail/):
  `page.rs` holds the route component `EventHubPage` and this route's chrome;
  `hero_countdown.rs` `event_hub_view`, the hub body; `mission_dossier.rs` one mission card;
  `slotting_selector.rs` `OrbatSelector`; `squad_pane.rs`, `seat_row.rs`, `assign_picker.rs` and
  `reservation_actions.rs` the squad, the seats, the member picker and the footer;
  `registration_access/` what the viewer may register for and why. The folder's
  [README](/apps/website/frontend/src/v2/pages/operations/event_detail/README.md) and the
  [registration access README](/apps/website/frontend/src/v2/pages/operations/event_detail/registration_access/README.md)
  describe each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/operations/event_detail/README.md#routes). The
  sidebar has no entry for it; the schedule, the dashboard banner, the deployments page and
  direct links lead here.
- Related: the [event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md),
  which renders the same hub body in its detail column; the
  [ORBAT selection page](/documentation_v2/website/frontend/pages/operations/orbat_selection/orbat_selection_page.md),
  which mounts the same selector for one mission; the
  [event manager page](/documentation_v2/website/frontend/pages/administration/event_manager/event_manager_page.md),
  where administrators set an event's missions, access policies and pools; the
  [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [operations domain](/apps/website/api_v2/src/operations/README.md) and its
  [reservation services](/apps/website/api_v2/src/operations/services/event_reservations/README.md).

## Behaviour

### Loading the hub

1. The page body sits in `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows
   the session states of the README's
   [States](/apps/website/frontend/src/v2/pages/operations/event_detail/README.md#states) until the
   viewer is signed in.
2. The signed-in half reads `:id`, fetches the event's hub and shows "Loading…", then
   "Failed to load data." or the hub. An event the viewer may not see fails like a missing one.
3. The route wraps the hub body in its own chrome: the topographic backdrop, a scroll surface at
   most 64rem wide, and the " All Operations" link back to `/events`. The schedule renders the
   same body without this chrome.
4. Every change the viewer makes in the slotting below fetches that mission's ORBAT again and
   then the whole hub, so the places, the notices and the counts stay live. The refetch rebuilds
   the hub, which resets each selector's faction and squad tabs.

### The hero and the places

1. The hero shows "Operation Hub", the event's name (its `name_override`, else
   "Untitled Operation"), "T-MINUS " and the time left as one rounded unit (fixed at render, and
   "T-MINUS LIVE NOW" once the start has passed), the local start time, the "Briefing" and two
   chips: " TS3: ts.tbdevent.eu", fixed text for the unit's voice server, and the modpack.
2. The modpack chip names the modpack the event binds, found in the modpack list, or the current
   modpack when the event binds none; it links to the modpack's workshop page and is left out when
   the fetch fails.
3. A viewer admitted only by squad or slot policies is a partial viewer: the API sends neither the
   event briefing nor the missions and seats their policies do not admit. The hero then reads
   "The operation briefing is shown only to participants the operation's own access policy
   admits." and the Places panel adds "You see only the missions and seats open to you, and not
   the operation briefing."
4. The Places panel shows each reservation pool of the event (member, guest and open places), with
   its state ("Open now · …", "Not open yet · …", "Closed · this pool has no places" or
   "Full · …"), its opening time in the viewer's zone beside UTC, "Your pool" on the viewer's own
   pool, and the places left in the event or "No operation-wide limit". A participant holds one
   place for the whole event, taken from their own pool first and then from the open pool, never
   past the event-wide limit.

### The mission dossiers

1. Under "Mission Dossiers", one card per mission in start order, numbered "Mission 1", "Mission
   2" and on: the title; terrain, game mode ("COOP", "PvP", "Zeus" or the raw value) and local
   start time; a "Terrain:" badge; the viewer's reservation state in capitals and
   "Attendance: <state>" when they have them; "<filled>/<total> slots filled"; and a " Mission
   Planner" button that is always disabled, titled "2D mission planner — coming soon".
2. The card's standing notices tell the viewer where they stand: registered, holding a place
   without a seat, their position on the waiting list, a released signup with its reason and time,
   no seat open to them under the access policies, or a Discord membership still being verified.
3. "Mission Briefing" shows the mission's briefing, or "No briefing provided." when it is blank or
   whitespace; the event briefing follows the same rule.
4. "Faction Dossiers" shows one card per faction, western sides first, then eastern, then
   independent, then the rest: three grey uniform silhouettes and, when the mission serves items
   for that faction, its "Armory" with each item's quantity or "∞".
5. With no missions the hub reads "No missions have been added to this operation yet."

### Taking a seat

1. The selector fetches the mission's ORBAT ("Loading ORBAT…"); a mission without slots reads
   "No ORBAT slots defined for this mission." With more than one faction, faction tabs head the
   squad list, sorted like the faction cards. Each squad row shows a lock when a leader holds the
   squad, its callsign and its fill count, red when full; the first squad is picked until the
   viewer picks another.
2. A seat row shows the occupant, or "Available" (or "Selected") on a free seat the viewer may
   take, "Reserved" in a squad another leader holds, or a lock with the policy that restricts it
   ("Restricted by this seat's access policy", "… this squad's …", "… the operation's …"). Only a
   free seat that admits the viewer can be selected.
3. The footer names the viewer's signup ("You are registered for this mission.", "You are on the
   waiting list, position N.") or what to do next ("Select an open slot to deploy.",
   "This squad is reserved by a leader.", "Assign members to fill this squad.").
4. "Register for Deployment" sends the selected seat. It shows while a place is free for the
   viewer and they may register, and stays disabled until a seat is selected. Success toasts
   "Registered for deployment".
5. "Join waiting list" registers without a seat. It shows when a seat admits the viewer but no
   place or seat is left, or when the last refusal pointed at the waiting list. The API holds a
   place without a seat when one is free ("Registered: a place is held for you without a seat")
   and waitlists the viewer otherwise ("Added to the waiting list").
6. A viewer who holds a place without a seat picks a free seat and registers for it.
7. "Withdraw", or "Leave waiting list" for a waiting viewer, releases the signup
   ("Withdrawn from mission").
8. A refused registration shows its sentence above the buttons: each refusal code the API names
   has its own wording, times show in the viewer's zone beside UTC, no sentence names another
   participant, and any other refusal shows the API's own sentence.

### Squads and the waiting list for leaders

1. A viewer with the `leader` [role](/documentation_v2/glossary/n_to_z.md#role) or above sees
   " Reserve Squad" on a free squad; success toasts "Reserved <squad>". A held squad shows
   "Reserved by <name>" (or "a leader"), and its reserver or an administrator sees "Release"
   ("Squad released").
2. The squad's reserver or an administrator assigns and clears its seats: "Assign" opens the
   member picker under the seat, which searches the member directory on every keystroke
   ("Search members…", "No matching members."); picking a member toasts "Assigned <name>", and
   "Clear" on a filled seat toasts "Slot cleared". For everyone else a held squad is read-only.
3. A leader or administrator sees "Promote from waiting list" on each mission card. It seats the
   earliest eligible waiting participants and reports "Seated 1 participant from the waiting
   list.", "Seated N participants from the waiting list." or "Nobody on the waiting list could
   be seated."; an `EVENT_FULL` refusal reads "No place is free for anyone waiting: the operation
   or its pools are full."
4. Every leader check reads the signed-in session's role, so a browse-mode session never gets a
   leader control.

### Known discrepancies

- The footer offers "Register for Deployment" and "Join waiting list" on an event whose
  registration is locked, or whose status is live, completed or cancelled
  (`reservation_footer` in
  `apps/website/frontend/src/v2/pages/operations/event_detail/reservation_actions.rs`); the API
  refuses with "registration is locked; an admin must assign you" or "registration is closed for
  this operation" (`require_registration_open` in
  `apps/website/api_v2/src/operations/services/event_reservations/mutation_authority.rs`), and the
  footer shows that sentence only after the click.
- A failed ORBAT fetch reads "No ORBAT slots defined for this mission.", the text of an empty
  ORBAT (`OrbatSelector` in
  `apps/website/frontend/src/v2/pages/operations/event_detail/slotting_selector.rs`), while the API
  answers 404 "mission not found" for a mission the viewer may not see (`get_orbat` in
  `apps/website/api_v2/src/operations/handlers/orbat_view.rs`).
- The hero prefixes every countdown with "T-MINUS ", so a started event reads "T-MINUS LIVE NOW"
  (`event_hub_view` in
  `apps/website/frontend/src/v2/pages/operations/event_detail/hero_countdown.rs`); the dashboard
  drops the prefix for `LIVE NOW`.
- The modpack chip links to "#" when the modpack has no workshop address and still opens it in a
  new tab (`event_hub_view`, same file).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/operations/event_detail/README.md#data)
lists each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/events/{id}` (`get_event` in
  `apps/website/api_v2/src/operations/handlers/event_hub.rs`), for any signed-in member: the event
  and each attached mission's dossier in start order, projected for the viewer's access in one
  read-only snapshot. A viewer the event policy admits sees every mission; a partial viewer gets
  the summary without the briefing and only the missions, seats and factions their policies
  admit; a viewer who may see nothing gets 404 "event not found". The status is the effective
  status the [event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
  describes. Each dossier carries the viewer's own reservation, attendance, seat, waiting
  position and released signup, and the event carries the pools and the viewer's access.
- `GET /api/v1/event-missions/{emid}/orbat` (`get_orbat` in
  `apps/website/api_v2/src/operations/handlers/orbat_view.rs`): the mission's `orbat_slots` rows
  grouped by faction and squad, with occupant and reserver names and each seat's eligibility for
  the viewer; a partial viewer gets only the seats their policies admit.
- `POST /api/v1/event-missions/{emid}/register` (`register_for_event_mission` in
  `apps/website/api_v2/src/operations/handlers/slot_registration.rs`), any signed-in member, body
  `{ "slot_id": … }` with `""` for a seatless place: refused with 409 while the event is not
  scheduled or open, and with 403 while registration is locked, unless the caller is an
  administrator. Under the event's lock it plans the claim against access, pools, capacity and
  squad holds (an administrator passes a squad hold), and records a seat, a seatless place or a
  waiting entry, with an audit row. A refusal carries a code; one that waits on Discord
  membership verification queues that verification first.
- `DELETE /api/v1/event-missions/{emid}/register` (`withdraw_from_event_mission`, same file):
  releases the caller's seats and signup on the mission, keeps the registration as `withdrawn`,
  releases places no longer used, and promotes the earliest eligible waiting participants in the
  same transaction; 404 "not registered" when there is nothing to release.
- `POST /api/v1/event-missions/{emid}/waitlist/promote` (`promote_waitlisted_participants` in
  `apps/website/api_v2/src/operations/handlers/waitlist_promotion.rs`), `leader` and above: seats
  the earliest eligible waiting participants in queue order, each with an actual seat and a place;
  the caller cannot choose who; refused while registration is locked.
- `POST /api/v1/event-missions/{emid}/squads/reserve` and `…/squads/release` (`reserve_squad` and
  `release_squad` in `apps/website/api_v2/src/operations/handlers/slot_assignment.rs`), `leader`
  and above: hold a squad (409 "squad is already reserved") and release it (403 "only the reserver
  or an admin can release this squad").
- `PUT` and `DELETE /api/v1/event-missions/{emid}/slots/{slotId}/assign` (`assign_slot` and
  `clear_slot`, same file), `leader` and above, and only for the squad's reserver or an
  administrator (403 "reserve this squad to manage its slots"): seat a member after checking
  their eligibility, pool, capacity and account, or clear the seat; clearing keeps the member's
  place until they withdraw.
- `GET /api/v1/members?q=…` (`search_members` in
  `apps/website/api_v2/src/operations/handlers/orbat_view.rs`), `leader` and above: members who
  are not banned, whose username or Discord handle contains the text, 20 per page.
- `GET /api/v1/modpacks` (`list_modpacks` in
  `apps/website/api_v2/src/community_content/handlers/modpack_catalog.rs`): every modpack, the
  current one first, which the chip searches for the event's modpack; `GET
  /api/v1/modpacks/current`: the modpack flagged current.

The page stores nothing in the browser.

## Design

- The hub is one column: the hero card, the Places panel, then the mission cards. Each mission
  card holds its header, notices, briefing, faction cards and, last, the selector: a 240px sidebar
  of faction tabs and squads beside the squad's seat list, with the footer across the bottom.
- The faction cards draw grey silhouettes where uniform artwork would go; the page shows no map
  thumbnail, weather, commander's intent, loadout preview, vehicle roster or objective list,
  since the dossier carries none of them.
- No blueprint set exists for this page. The event schedule's
  [operations schedule blueprint](/documentation_v2/website/frontend/pages/operations/schedule/visual_references/operations_schedule_blueprint/README.md)
  shows the hub as a banner, one briefing and mission tabs over an ORBAT; the built hub stacks
  every mission as its own card.
- A design-phase layout sketch planned a hero with a "Connect TS3" button, the Places panel,
  each mission's dossier with its waiting-list and release notices, a "Promote from waiting list"
  control, and the ORBAT selector with faction tabs, locked seats and the footer buttons. The
  built hub follows it, with two differences: the voice server is a plain chip, not a button,
  and the ORBAT sits inside each mission card rather than in one selector below the dossiers.

## Open work

- [T-137 — Discord platform rework](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (deferred, no plan): Discord bot flows for slot confirmation and reminders, which would
  confirm in Discord the seats taken on this page.

## Decisions

- One hub renderer: `event_hub_view` serves `/events/:id` and the schedule's detail column, and
  `OrbatSelector` serves the hub and the ORBAT selection page, so the three views cannot drift
  apart.
- The page offers what the API returned to this viewer: the standings, seat eligibility and
  places are pure functions of the hub the API sent, an unknown pool state reads as closed and an
  unknown seat standing as restricted, so the page never offers what the API would refuse on
  access grounds (`an_unknown_pool_state_reads_as_closed` and
  `restricted_seats_are_closed_and_say_why` in
  `apps/website/frontend/src/v2/pages/operations/event_detail/registration_access/tests/registration_access.rs`).
- Joining the waiting list is a registration without a seat: the API decides between a held
  place and a waiting entry, so the page needs no second route.
- Only dossier facts render: every meta badge and briefing comes from the dossier, and a blank
  briefing says so rather than showing invented text
  (`meta_badges_are_all_dossier_derived` and
  `a_cleared_briefing_renders_the_empty_state_and_never_the_invented_lore` in
  `apps/website/frontend/src/v2/pages/operations/event_detail/tests/event_hub.rs`).
- A squad hold makes the squad its reserver's to fill: other members cannot take its seats, and
  only the reserver or an administrator assigns and clears them, in the page and in the API
  alike (`require_squad_management` in
  `apps/website/api_v2/src/operations/services/event_reservations/mutation_authority.rs`).
