**Status:** live

# ORBAT selection page

The `/events/:id/missions/:emid/orbat` page in the
[operations](/documentation_v2/glossary/n_to_z.md#operations) section: one
[mission](/documentation_v2/glossary/g_to_m.md#mission)'s slotting on a page of its own, for a link that
should open straight onto the seats, such as a pinned Discord message or the "Modify Assignment"
link on the deployments page. It mounts the same [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat)
selector the event hub shows under each mission.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/operations/orbat_selection/`](/apps/website/frontend/src/v2/pages/operations/orbat_selection/):
  `page.rs` holds the route component `OrbatSelectionPage`, the event fetch, the mission lookup
  and the mounted selector. The folder's
  [README](/apps/website/frontend/src/v2/pages/operations/orbat_selection/README.md) describes it.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/operations/orbat_selection/README.md#routes). The
  sidebar has no entry for it; the deployments page's "Modify Assignment" link
  (`apps/website/frontend/src/v2/pages/operations/deployments/active_orders.rs`) and direct links
  lead here.
- Related: the [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md),
  which owns `OrbatSelector`, `MissionStanding` and `standing_notices` and describes the slotting
  in full; the [deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md),
  which links here.

## Behaviour

1. The page body sits in `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows
   the session states of the README's
   [States](/apps/website/frontend/src/v2/pages/operations/orbat_selection/README.md#states) until
   the viewer is signed in.
2. The signed-in half reads `:id` and `:emid`, fetches the event's hub and shows "Loading…".
3. It looks the mission up in the hub by its event mission id. When the hub holds it, the page
   shows a back link to `/events/{id}` named after the event's `name_override` (or "Operation"),
   the mission's title as the heading, "Select your faction, squad, and slot, then register for
   deployment.", and the viewer's standing notices on the mission.
4. When the fetch fails, or the event does not list that mission (a stale link, a mission detached
   since, or one the viewer may not see), the page still renders: the back link reads "Operation",
   the heading "Order of Battle", no standing notice shows, and the standing withholds no action,
   so the API decides what the viewer may do. The page never shows "Failed to load data.".
5. Under the header sits `OrbatSelector` for `:emid`, unchanged from the hub: faction tabs, squads,
   seats, squad holds, member assignment and the footer with "Register for Deployment",
   "Join waiting list" and "Withdraw". The
   [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
   describes each of them. The selector mounts only when the path carries a mission id.
6. Every change in the selector fetches the event again, so the notices and the footer follow the
   viewer's new standing; the refetch rebuilds the selector and resets its tabs.
7. The page shows only the selector's part of the hub: no Places panel, no briefing, no faction
   dossiers, and no "Promote from waiting list", which sits on the hub's mission card.

### Known discrepancies

- A failed hub fetch and a mission the viewer may not see both render the generic page with an
  unrestricted standing (`OrbatSelectionInner` in
  `apps/website/frontend/src/v2/pages/operations/orbat_selection/page.rs`). For a mission of an
  event the viewer may not see, the selector's own ORBAT fetch fails with 404 "mission not found"
  (`get_orbat` in `apps/website/api_v2/src/operations/handlers/orbat_view.rs`) and reads
  "No ORBAT slots defined for this mission.", so the viewer is not told the link is closed to
  them.
- The back link names an event without a `name_override` "Operation", where the hub it leads to
  calls it "Untitled Operation"
  (`event_hub_view` in `apps/website/frontend/src/v2/pages/operations/event_detail/hero_countdown.rs`).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/operations/orbat_selection/README.md#data)
lists the calls. Server-side:

- `GET /api/v1/events/{id}` (`get_event` in
  `apps/website/api_v2/src/operations/handlers/event_hub.rs`): the event's hub projected for the
  viewer's access, which the page reads for the event's name, the mission's title and the
  viewer's standing; 404 "event not found" for an event the viewer may not see.
- The selector's calls and their server-side meaning are the event hub page's
  ([Data](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md#data)):
  `GET /api/v1/event-missions/{emid}/orbat`, registration and withdrawal, squad holds, seat
  assignment and the member directory.

The page itself writes nothing and stores nothing in the browser.

## Design

- A padded page inside the navigation frame, not full bleed: a column at most 64rem wide with the
  back link, the header and the selector.
- No blueprint set exists for this page. A design-phase layout sketch planned a
  "Back to Operation Dossier" link, a "SELECT SQUAD ASSIGNMENT — MISSION 1" heading and a
  full-bleed squad and slot roster. The built page names the back link after the event, heads it
  with the mission's title, and shows the selector at the hub's width rather than full bleed.

## Open work

None. No open ticket in `.ai/tickets/` changes this page; the known discrepancies above have no
ticket yet.

## Decisions

- One slotting implementation: the page mounts the hub's `OrbatSelector` rather than a roster of
  its own, so a seat behaves the same wherever it is taken.
- A stale link renders instead of failing: the mission and the standing are looked up in the
  fetched event, and a mission the event does not list gets a standing that withholds nothing,
  leaving the decision to the API.
- The page keeps only what slotting needs: the briefing, places and faction dossiers stay on the
  hub, one click away through the back link.
