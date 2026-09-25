# ORBAT selection page

One [mission](/documentation_v2/glossary.md#mission)'s slotting on a page of its own, reachable
directly by link: the same [ORBAT](/documentation_v2/glossary.md#orbat) selector the
[event](/documentation_v2/glossary.md#event) hub page embeds, under the mission's heading and the
viewer's standing on it.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/orbat_selection/
├── mod.rs   the module tree; re-exports `OrbatSelectionPage`
└── page.rs  the route component: the event fetch, the mission lookup and the mounted selector
```

## How it works

`OrbatSelectionPage` renders inside `AuthGate`. The signed-in half reads `:id` and `:emid`, fetches
the event and looks the mission up in it by `event_mission_id` for the heading and a
`MissionStanding` (reservation state, waiting position, seat eligibility and place outlook). A path
naming a mission the event does not carry, or a failed fetch, still renders, with the generic
heading and `MissionStanding::unlisted`, which withholds no action and leaves the decision to the
[API](/documentation_v2/glossary.md#api). The page has no roster of its own: it mounts
`OrbatSelector` from the event hub page unchanged, and only when the path carries a mission id; the
selector's change callback fetches the event again, so the notices stay live.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/events/:id/missions/:emid/orbat` | `OrbatSelectionPage` | route tier `none`; the data renders only for a signed-in viewer | padded inside the navigation frame; breadcrumb Operations / ORBAT Selection |

## Data

- `GET /api/v1/events/{id}`: the event, read as `EventHub`, for the event name, the mission title
  and the viewer's standing.
- The mounted `OrbatSelector` makes the event hub page's slotting calls:
  `GET /api/v1/event-missions/{emid}/orbat` and the registration, squad and seat mutations.
- The page reads the session from the `AuthStore` context and the path parameters from the router,
  and writes nothing itself. The fetch runs in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| loaded | a back link to `/events/{id}` named after the event, the mission title, "Select your faction, squad, and slot, then register for deployment.", the standing notices and the selector |
| unknown mission or failed fetch | the same layout with the back link "Operation", the heading "Order of Battle" and no standing notices |

## Boundaries

- Depends on: `OrbatSelector`, `MissionStanding` and `standing_notices` from
  `apps/website/frontend/src/v2/pages/operations/event_detail/`; `crate::v2::core::api` (the
  `api_get` client, `EventHub`); `crate::v2::core::ui` (`AuthGate`, `MaterialIcon`); the
  `AuthStore` context and `use_params_map`.
- Used by: the `/events/:id/missions/:emid/orbat` route in
  `apps/website/frontend/src/app_routes.rs`; the
  [deployments](/documentation_v2/glossary.md#deployment) page's active orders link to it
  (`apps/website/frontend/src/v2/pages/operations/deployments/active_orders.rs`).
- Rules: the slotting tree is the event hub page's selector, never a second implementation; the
  mission and the standing are looked up in the fetched event, so a stale link renders instead of
  failing.

## Related documentation

- [ORBAT selection page](/documentation_v2/website/frontend/pages/operations/orbat_selection/orbat_selection_page.md)
  — the page's behaviour, stale links and design.
- [Event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md)
  — the event dossier and its slotting, which this page mounts.
