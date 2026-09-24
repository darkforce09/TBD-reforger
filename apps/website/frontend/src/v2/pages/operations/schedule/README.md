# Event schedule page

The `/events` page: every upcoming [event](/documentation_v2/glossary.md#event) in a master list,
with the full hub of the selected event beside it.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/schedule/
├── mod.rs           the module tree; re-exports `EventSchedulePage`
├── page.rs          the route component: list fetch, selection, hub fetch and the split pane
├── tests/           unit tests that keep briefings on the shared hub body
└── upcoming_ops.rs  one event card, and the readers for the untyped event row
```

## How it works

`EventSchedulePage` renders its content inside `AuthGate`, so a viewer who is not signed in sees
the sign-in prompt instead. The signed-in half fetches the event list once and lays it out in a
`SplitPane` whose master column is headed "Upcoming Ops". A card click writes the `picked` signal;
the `selected_id` memo resolves it, or the first event when nothing is picked, and keys a second
fetch for that event's hub. The hub result carries the id it was fetched for, and the detail
column renders a hub only when that id is the selected one, so one event's hub never shows under
another event's selection. The detail column is `event_hub_view` from the event hub page, inline
slotting included; its change callback fetches the hub again.

`upcoming_ops.rs` draws each card from the untyped event row: the local start time, the status badge
(`open`, `locked`, `live`, `completed`, `cancelled`, any other value neutral), the name or "Untitled
Operation", the [mission](/documentation_v2/glossary.md#mission) and
[slot](/documentation_v2/glossary.md#slot) counts, a countdown or `LOCKED` when registration is
locked, and a fill bar driven by the server's `percent`, clamped to 0 to 100, since `total_slots` is
zero until missions are attached. The list shows in the order the
[API](/documentation_v2/glossary.md#api) returns it: there is no date picker, no week or month
window, no archive of past events and no after-action link.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/events` | `EventSchedulePage` | route tier `none`; the data renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Operations › Event Schedule |

## Data

- `GET /api/v1/events`: the list, read as `Paginated<serde_json::Value>` and sent without query
  parameters, so the API's default `scope` (`upcoming`) and first page apply; each card reads `id`,
  `name_override`, `start_time`, `status`, `registration_locked`, `mission_count`, `filled`,
  `total_slots` and `percent`.
- `GET /api/v1/events/{id}`: the selected event's `EventHub`, rendered by `event_hub_view`, whose
  slotting calls are the event hub page's.
- The page reads the session from the `AuthStore` context and writes nothing itself. Both fetches
  run in the browser build only; a native build resolves the list to `None`.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| list loading | "Loading…" |
| list failed | "Failed to load data." |
| no events | "No upcoming operations scheduled." beside "Select an operation to view its hub." |
| hub loading | "Loading operation…", also while the previous event's hub is still held |
| hub failed | "Could not load this operation's hub." |
| loaded | the selected event's hub, with its inline slotting |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get` client, `EventHub`, `Paginated`),
  `crate::v2::core::ui` (`AuthGate`, `SplitPane`, `SplitPaneEmpty`, `MaterialIcon`, `badge_class`,
  `cn`), `crate::v2::core::utils` (countdown and local date formatting), the `AuthStore` context,
  and `event_hub_view` from `apps/website/frontend/src/v2/pages/operations/event_detail/`.
- Used by: the `/events` route in `apps/website/frontend/src/app_routes.rs`;
  `event_schedule_source` in `apps/website/frontend/src/v2/core/test_support/pins.rs` reads its
  three source files.
- Rules: briefings render through `event_hub_view`, whose blank-briefing rule trims whitespace
  (`schedule_briefing_empty_check_stays_trim_aligned` in `tests/schedule.rs`); the detail column
  never shows a hub fetched for another event.

## Related documentation

- [Event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
  — the page's behaviour and design.
