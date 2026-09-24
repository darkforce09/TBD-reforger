**Status:** live

# README template: page

**When to use:** a folder under `apps/website/frontend/src/v2/pages/` that holds a route component.
The [README standard](/documentation_v2/standards/readme_standard.md) defines every rule this
template follows; the page kind adds Routes, Data and States.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there.

````markdown
# <Page name, in plain words: no path, no backticks>

<One to three sentences: what the viewer does on this page.>

## Contents

```text
<repository path of the folder>/
├── mod.rs     <what the module declares and re-exports>
├── page.rs    <the route component and what it owns>
├── <file>     <what it is for: a lowercase phrase, no closing period>
└── tests/     <what the tests pin>
```

## How it works

<How the page is put together: the gates it sits behind, the resources and signals it owns, how a
click or a fetch moves it on, and the invariants that span files.>

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| <path from app_routes.rs> | <component> | <tier from router.rs, and any gate inside the page> | <full-bleed and chromeless flags, breadcrumb> |

## Data

- <METHOD /api/v1/path>: <the DTO it reads or sends, and what the page does with it>
- <the context and storage the page reads, and what it writes>

## States

| State | What the viewer sees |
|---|---|
| <state> | <the text or view on screen, quoted as the code writes it> |

## Boundaries

- Depends on: <the core modules, sibling pages and DTOs the page uses>
- Used by: <the route table, and any other page or test that reads the page>
- Rules: <the invariants a change here must keep>

## Related documentation

- [<feature doc title>](/documentation_v2/website/frontend/pages/<area>/<page>/<doc>.md) — <what
  it covers>
````

## Worked sample

Written from `apps/website/frontend/src/v2/pages/operations/schedule/`. The sample sits in a fenced
block, so no gate reads it as a README; the folder's own README.md is written from the same code and
may differ.

````markdown
# Event schedule page

The `/events` page: every upcoming [event](/documentation_v2/glossary.md#event) in a master list,
with the full hub of the selected event beside it.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/schedule/
├── mod.rs           declares the page and card modules and re-exports `EventSchedulePage`
├── page.rs          the route component: list fetch, selection, hub fetch and the split pane
├── tests/           the pin that keeps briefings on the shared hub view
└── upcoming_ops.rs  one event card, and the readers for the untyped event row
```

## How it works

`EventSchedulePage` renders its content inside `AuthGate`, so a viewer who is not signed in sees
the sign-in prompt instead. The signed-in half fetches the event list once and lays it out in a
`SplitPane`. A card click writes the `picked` signal; the `selected_id` memo resolves it, or the
first event when nothing is picked, and keys a second fetch for that event's hub. The hub result
carries the id it was fetched for, and the detail column renders a hub only when that id is the
selected one, so one event's hub never shows under another event's selection. `upcoming_ops.rs`
draws each card from the untyped event row: local start time, status badge,
[mission](/documentation_v2/glossary.md#mission) and slot counts, a countdown or `LOCKED`, and a
fill bar driven by the server's `percent`, clamped to 0 to 100.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/events` | `EventSchedulePage` | route tier `none`; the data renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Operations › Event Schedule |

## Data

- `GET /api/v1/events`: the list, read as `Paginated<serde_json::Value>`; each card reads `id`,
  `name_override`, `start_time`, `status`, `registration_locked`, `mission_count`, `filled`,
  `total_slots` and `percent`.
- `GET /api/v1/events/{id}`: the selected event's `EventHub`, rendered by `event_hub_view`, whose
  change callback fetches it again.
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
  `crate::v2::core::ui` (`AuthGate`, `SplitPane`, `SplitPaneEmpty`, `MaterialIcon`, the badge
  classes), `crate::v2::core::utils` (countdown and local date formatting), the `AuthStore` context,
  and `event_hub_view` from `apps/website/frontend/src/v2/pages/operations/event_detail/`.
- Used by: the `/events` route in `apps/website/frontend/src/app_routes.rs`; the source pins in
  `apps/website/frontend/src/v2/core/test_support/pins.rs` read its three source files.
- Rules: briefings render through `event_hub_view`, which `tests/schedule.rs` pins; the detail
  column never shows a hub fetched for another event.

## Related documentation

- [Event schedule page](/documentation_v2/website/frontend/pages/operations/schedule/event_schedule_page.md)
  — the page's behaviour and design.
````
