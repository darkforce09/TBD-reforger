# Operation Dossier & Slotting (`/events/:id`)

One operation: when it starts, what its missions are, which places the viewer can have, and which
slot the caller takes in each mission.

## Architecture
- **`page.rs`**: the route component. Reads the operation id from the path, fetches
  `GET /events/:id`, and wraps the hub body in this route's chrome — the topographic backdrop,
  the scroll surface and the link back to the schedule.
- **`hero_countdown.rs`**: the hub body, shared with the schedule's detail column. The hero
  section — operation name, T-minus clock, local start time, briefing, voice-server chip and the
  modpack link — the Places panel, and the list of mission dossiers under it, each handed the
  viewer's standing on that mission. A partial viewer is told the briefing is withheld from them
  rather than that there is none.
- **`mission_dossier.rs`**: one mission card — heading, terrain/mode/time line, meta badges,
  registration state and fill counts, the leader's waiting-list promotion, the notices about the
  viewer's standing, briefing, faction grid and the slotting selector — plus the label rules, the
  briefing empty rule and the modpack-fetch choice the hero reads with.
- **`faction_armory.rs`**: one faction's card — the uniform silhouettes and the armory the
  mission serves for that faction — and the side ordering every faction list on the page uses.
- **`slotting_selector.rs`**: the order-of-battle selector. Fetches
  `GET /event-missions/:emid/orbat`, resolves the active faction and squad, and renders the
  faction tabs, the squad list, the slot pane and the footer action bar; owns the refusal notice
  of the last failed registration.
- **`squad_pane.rs`**: the selected squad — its reservation header with reserve and release, its
  slot list, and the squad-hold rules the seat rows, the footer line and the register button read.
- **`seat_row.rs`**: one slot row — role, loadout and tag, the occupant or availability marker,
  the manager's assign and clear controls, and for a seat whose policy does not admit the viewer
  a lock with the policy that restricts it (the operation's, the squad's or the seat's own). Only a
  free seat that admits the viewer is selectable.
- **`reservation_actions.rs`**: the footer bar — the viewer's signup line (registered, a place held
  without a seat, or the waiting-list position), the refusal notice worded by its reason, and the
  withdraw (or leave the waiting list), join-the-waiting-list and register buttons. Joining the
  waiting list is a registration without a seat, which the backend waitlists when no place is free;
  a viewer holding a place without a seat may pick a free seat and register for it.
- **`assign_picker.rs`**: the inline member typeahead a squad manager fills an empty slot with.
- **`registration_access/`**: what the viewer may register for, and why —
  - `place_outlook.rs`: whether a place can be had now — held already, available, opening later
    or exhausted — from the viewer's own pool, the open pool and the operation-wide remainder;
  - `mission_standing.rs`: the viewer's standing on one mission (reservation state, waiting
    position, a released signup's reason and time, eligibility, pending verification), which
    actions it offers, and the standing notices on the mission card;
  - `places_panel.rs`: the Places panel — each pool's availability with its opening time in the
    viewer's zone and in UTC and its closed reason, the places left in the operation, the viewer's
    own pool, and the partial-view and pending-verification notices;
  - `refusal_notices.rs`: every registration refusal the backend names (`ACCESS_POLICY`,
    `MEMBERSHIP_VERIFICATION_REQUIRED`, `QUOTA_NOT_OPEN`, `EVENT_FULL`, `MISSION_FULL`,
    `SEAT_NEEDED_BY_HOLDER`, `SEAT_TAKEN`, `SQUAD_HELD`, `NO_SEATS`, `REGISTRATION_CLOSED`,
    `ACCOUNT_UNAVAILABLE`, `DEPLOYMENT_REQUIREMENTS`) worded as a sentence, and which of them
    point at the waiting list;
  - `seat_eligibility.rs`: whether a seat admits the viewer, and the policy that restricts it;
  - `waitlist_promotion.rs`: the "Promote from waiting list" control a leader or administrator
    sees on each mission card.
- **`tests/event_hub.rs`**: the briefing empty rule and where it is bound, the meta badges, the
  modpack chip's fetch choice, the tier checks behind the slotting affordances, the reservation
  action truth table, and the avatar sink in the member picker.
- **`registration_access/tests/registration_access.rs`**: the place outlook, the mission standing
  and its offers, the refusal sentences, the seat restrictions, the Places panel lines and the
  promotion copy, all held against the captured dossier and order of battle.

## What the viewer sees depends on what they may see
- A viewer admitted only by squad or slot policies receives only the missions and seats open to
  them and no operation briefing; the page renders exactly that and says the view is partial.
- A seat whose policy does not admit the viewer is shown, marked restricted with the policy that
  restricts it, and cannot be selected.

## Not present in the legacy page
- **AO satellite preview and weather forecast**: the dossier header carries terrain, game mode
  and start time as text. There is no map thumbnail and no weather on this page.
- **Commander intent**: the mission dossier renders one authored briefing, and an explicit
  "no briefing provided" affordance when it is blank. There is no separate intent field.
- **Equipment loadout preview and vehicle allocations**: the faction card shows uniform
  silhouettes and the armory item list the mission serves. The dossier carries no loadout
  preview and no vehicle roster, so neither is rendered — not even as an empty state, which
  would advertise an authoring path that does not exist.
- **Objective lists**: same reason. There is no objectives field on the wire and no writer for
  one.
