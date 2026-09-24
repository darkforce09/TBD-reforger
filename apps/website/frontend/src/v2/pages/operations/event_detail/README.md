# Event hub page

The `/events/:id` page: one [event](/documentation_v2/glossary.md#event) with its start time,
briefing and places, a dossier for each attached [mission](/documentation_v2/glossary.md#mission),
and the inline [ORBAT](/documentation_v2/glossary.md#orbat) selector through which the viewer takes a
[slot](/documentation_v2/glossary.md#slot) or joins a waiting list.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/event_detail/
├── assign_picker.rs        the member typeahead a squad manager fills an empty slot with
├── faction_armory.rs       one faction's card (uniforms and armory) and the faction sort order
├── hero_countdown.rs       `event_hub_view`: the hub body shared with the schedule's detail column
├── mission_dossier.rs      one mission card, with its label, briefing and modpack-fetch rules
├── mod.rs                  the module tree; re-exports the page, hub body, selector and standing
├── page.rs                 the route component: the event fetch and this route's chrome
├── registration_access/    what the viewer may register for, and why: places, standing, refusals
├── reservation_actions.rs  the selector's footer: signup line, refusal notice, registration buttons
├── seat_row.rs             one slot row: occupant or availability, lock, assign and clear controls
├── slotting_selector.rs    `OrbatSelector`: the ORBAT fetch, faction tabs, squad list and slot pane
├── squad_pane.rs           the selected squad: reserve or release, its slot rows, squad-hold rules
└── tests/                  unit tests for the briefing rule, badges, modpack choice and tier checks
```

## How it works

`EventHubPage` renders inside `AuthGate`. The signed-in half reads `:id`, fetches the `EventHub`
and wraps `event_hub_view` in this route's chrome: the topographic backdrop, the scroll surface and
the " All Operations" link back to `/events`. The schedule page renders the same `event_hub_view`
in its detail column, so the two never drift apart.

```text
EventHubPage ─► AuthGate ─► fetch EventHub ─► event_hub_view
                                ├─ hero: name, countdown, start time, briefing, chips
                                ├─ places_panel (registration_access/)
                                └─ mission_dossier × N ─► OrbatSelector
                                                            ├─ faction tabs and squad list
                                                            ├─ squad_pane ─► seat_row × N
                                                            └─ reservation footer
a mutation ─► the selector refetches its ORBAT ─► on_change ─► the page refetches the EventHub
```

The hub builds each mission's `MissionStanding` from the dossier and hands it to the card and its
selector, so the offered actions follow what the [API](/documentation_v2/glossary.md#api) returned
to this viewer. A viewer admitted only by squad or slot policies receives only the missions and
seats open to them and no event briefing, and the page says the view is partial. A seat whose policy
does not admit the viewer shows a lock naming that policy and cannot be selected; only a free seat
that admits the viewer is selectable. Joining the waiting list is a registration without a seat,
which the API waitlists when no place is free, and a viewer who holds a place without a seat may
pick a free seat and register for it.

The tier checks read the session's [role](/documentation_v2/glossary.md#role) as memos through
`has_min_role_authed`: the `leader` role reserves a free squad and promotes from the waiting list; a
squad's reserver or an administrator releases it and assigns or clears its seats, and a reserved
squad is read-only for everyone else. Each card renders only what the dossier carries: terrain, game
mode and start time as text, the meta badges, fill counts, the briefing (with "No briefing
provided." when it is blank or whitespace) and the faction cards; there is no map thumbnail,
weather, commander intent, loadout preview, vehicle roster or objective list. The " Mission Planner"
button is always disabled, titled "2D mission planner — coming soon". A refetch rebuilds the
subtree, which resets the selector's faction and squad tabs.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/events/:id` | `EventHubPage` | route tier `none`; the data renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Operations › Event Hub |

## Data

- `GET /api/v1/events/{id}`: the event, read as `EventHub`; every slotting mutation fetches it
  again.
- `GET /api/v1/modpacks` (`DataEnvelope<ModpackDto>`, filtered to the event's `modpack_id`) when
  the event names a modpack, otherwise `GET /api/v1/modpacks/current` (`ModpackDto`): the modpack
  chip, left out when the fetch fails.
- `GET /api/v1/event-missions/{emid}/orbat`: one mission's ORBAT, read as
  `DataEnvelope<OrbatSquad>`.
- `POST /api/v1/event-missions/{emid}/register` with `{ "slot_id": … }`, read as
  `ReservationResponse`: register for the picked seat, or with an empty id for a seatless place;
  `DELETE` on the same path withdraws or leaves the waiting list.
- `POST /api/v1/event-missions/{emid}/waitlist/promote`, read as `WaitlistPromotion`.
- `POST /api/v1/event-missions/{emid}/squads/reserve` and
  `POST /api/v1/event-missions/{emid}/squads/release`, each with `{ "squad": … }`.
- `PUT /api/v1/event-missions/{emid}/slots/{slotId}/assign` with `{ "discord_id": … }`, and
  `DELETE` on the same path to clear the seat.
- `GET /api/v1/members?q=…`: the assign picker's matches, read as `DataEnvelope<Member>`, the query
  URL-encoded on every keystroke.
- The page reads the session (the viewer's Discord id and role) from the `AuthStore` context and
  stores nothing in the browser. Every fetch and mutation runs in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load data." |
| loaded | the hero (the event name or "Untitled Operation", "T-MINUS " and the countdown, the local start time, "Briefing", the " TS3: ts.tbdevent.eu" chip, the modpack chip), the "Places" panel and "Mission Dossiers" |
| partial view | "The operation briefing is shown only to participants the operation's own access policy admits." in the hero, and "You see only the missions and seats open to you, and not the operation briefing." in the Places panel |
| no missions | "No missions have been added to this operation yet." |
| ORBAT loading | "Loading ORBAT…" |
| no ORBAT | "No ORBAT slots defined for this mission." |
| no squad picked | "Select a squad to view its slots." |
| footer line | the viewer's signup ("You are registered for this mission.", "You are on the waiting list, position N.") or "This squad is reserved by a leader.", "Assign members to fill this squad.", "Select an open slot to deploy." |
| footer buttons | "Withdraw" (or "Leave waiting list"), "Join waiting list", "Register for Deployment" |
| refused | the refusal sentence of the last failed registration, above the buttons |
| mutation done | a toast: "Registered for deployment", "Added to the waiting list", "Registered: a place is held for you without a seat", "Withdrawn from mission", "Reserved <squad>", "Squad released", "Assigned <name>", "Slot cleared" |
| mutation failed | a toast: "Could not withdraw", "Could not reserve squad", "Could not release squad", "Could not assign member", "Could not clear slot" |
| assign picker | the "Search members…" field, and "No matching members." when nothing matches |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get`, `api_post_ok`, `api_put` and `api_delete`
  client, the `event_registration` endpoint helpers, `EventHub`, `EventMissionDossier`,
  `OrbatSquad`, `OrbatSlot`, `ModpackDto`, `Member`, `DataEnvelope`), `crate::v2::core::auth`
  (`AuthStore`, `has_min_role_authed`, `Role`), `crate::v2::core::ui` (`AuthGate`, `MaterialIcon`,
  `cn`, `DEFAULT_AVATAR`, the toasts) and `crate::v2::core::utils` (countdown and local date
  formatting).
- Used by: the `/events/:id` route in `apps/website/frontend/src/app_routes.rs`; the schedule page
  in `apps/website/frontend/src/v2/pages/operations/schedule/` (`event_hub_view`); the standalone
  slotting page in `apps/website/frontend/src/v2/pages/operations/orbat_selection/`
  (`OrbatSelector`, `MissionStanding`, `standing_notices`); `event_hub_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs` reads its source files.
- Rules: `event_hub_view` is the one renderer of an event; a blank or whitespace briefing, the
  event's and each mission's, reads as "No briefing provided."
  (`a_cleared_briefing_renders_the_empty_state_and_never_the_invented_lore` and
  `operation_level_briefing_uses_the_same_empty_rule` in `tests/event_hub.rs`); every meta badge
  comes from the dossier (`meta_badges_are_all_dossier_derived`); the tier checks use the
  authenticated reactive role, so a browse-mode session is never a leader
  (`orbat_affordances_use_authed_reactive_role`).

## Related documentation

- [Event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
  — the page's behaviour and design.
