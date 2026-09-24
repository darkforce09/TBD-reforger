# Mortar calculator page

The `/tools/mortar` page: a fire position, a target and a mortar tube go in, and the firing solution
the [API](/documentation_v2/glossary.md#api) computes comes back. With an
[event](/documentation_v2/glossary.md#event) selected the solution is saved against it, and the page
reloads that event's saved fire missions.

## Contents

```text
apps/website/frontend/src/v2/pages/field_tools/mortar/
├── firing_solution.rs  the solution card: its five figures and whether they survive a reload
├── grid.rs             pure maths: the grid string, its inverse, the preview fit, integer grouping
├── map_picker.rs       the four coordinate fields and the preview with its line and markers
├── mod.rs              the module tree; re-exports `MortarCalculatorPage`
├── page.rs             `MortarCalculatorPage`: signals, both fetches, effects and the solve request
├── saved_fires.rs      saved-row types, the request body, restore rules, stored event, list panel
├── tests/              unit tests for the save round trip, grid string, preview and hydration
└── weapon_selector.rs  the tube and event pickers, and `WEAPONS`, the tubes the solver accepts
```

## How it works

`MortarCalculatorPage` renders inside `AuthGate`. The server solves the ballistics; nothing in this
folder computes a firing solution.

```text
event selected ──▶ POST /api/v1/fire-missions ───────▶ solution + stored row ──▶ "Saved"
no event       ──▶ POST /api/v1/fire-missions/solve ──▶ solution ──────────────▶ "Not saved"
```

With an event selected one request computes and saves, so the card never shows numbers that failed
to save, and the saved list refetches; without one the page solves only and says the result is
lost on reload. Both requests send the same body, with `event_id` left out rather than sent blank
when no event is selected.

The selected event is kept in `localStorage` under `tbd-mortar-event`, so a reload returns to it;
once the event list lands, a stored id that names no listed event is cleared. The saved fire
missions are fetched per event, and each batch is tagged with the event it was fetched for, so
nothing acts on a batch that belongs to another event. The newest saved row fills the form once per
event (`hydration_step`), and any row loads on a click. The four inputs are plain metric x and y
values, and `fmt_grid` stores each pair as the lossless string `x, y`, not a military grid
reference. `restore` reads a row's numeric coordinates first and falls back to those strings; a
row with neither restores nothing. The preview has no terrain behind it: it fits both points into a
square frame, centred between them and 1.6 times their larger span, so both markers stay on screen
at any separation.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/tools/mortar` | `MortarCalculatorPage` | route tier `none`; the calculator renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Field Tools / Mortar Calculator |

## Data

- `GET /api/v1/events`: read as `Paginated<EventOption>`, each event's `id`, `name_override` and
  `start_time`, for the event picker.
- `GET /api/v1/events/{id}/fire-missions`: the selected event's saved rows, read as
  `DataEnvelope<SavedFire>`.
- `POST /api/v1/fire-missions`: with an event selected; sends `weapon_system`, `fp_x`, `fp_y`,
  `tgt_x`, `tgt_y`, `fp_grid`, `target_grid` and `event_id`, and reads `SaveResponse`: the
  `FireSolution` and the stored `SavedFire`.
- `POST /api/v1/fire-missions/solve`: with no event; the same body without `event_id`, read as
  `FireSolution`.
- `localStorage` key `tbd-mortar-event`: read at mount, written when an event is picked, removed
  when none is picked or the stored id names no listed event.
- The page reads the `AuthStore` and the toast queue from context. The requests and the storage
  run in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| inputs | "Mortar Calculator", "Enter grid coordinates, pick a tube, and save the solution to an operation.", then "Weapon" (`M252 81mm`, `M821 81mm`, `2B14 82mm`, `M120 120mm`), "Operation" ("— none (not saved) —", or "<name> — <date>" with "Untitled Operation" for an unnamed event), "FP X", "FP Y", "TGT X" and "TGT Y" |
| button | "Calculate & Save" with an event, "Calculate Solution" without, "Computing…" while a request runs |
| no solution | "Firing Solution — <weapon>" over "Enter coordinates and calculate to see solution." |
| solution | "Saved — survives a reload" or "Not saved — lost on reload", then "Distance" in m, "Azimuth" in degrees, "Elevation" in mils, "Charge" and "TOF" in s; "—" for a charge or flight time an older saved row lacks |
| saved list | "Saved Fire Missions" with "Loading…" (also while a batch for another event is held), "Could not load saved fire missions.", "Nothing saved on this operation yet.", "Pick an operation to save and reload solutions.", or rows "<fp grid> → <target grid>" over "<n> m · <deg>° · <n> mils", newest first |
| toasts | "Firing solution saved to the operation", "Not saved — pick an operation to keep this solution", or on failure the API's message or "Could not compute firing solution" |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `api_post`, `api_error_message`, and
  `FireSolution`, `DataEnvelope` and `Paginated` from `dto/telemetry.rs` and `dto/common.rs`),
  `crate::v2::core::ui` (`AuthGate`, `PageHeader`, the toast queue),
  `crate::v2::core::utils::datefmt::format_short_date`, the `AuthStore` context, and the browser's
  `localStorage` through `web_sys`.
- Used by: the `/tools/mortar` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Mortar Calculator" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; the DOM oracle's `mortar` capture
  in `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules:
  - `WEAPONS` mirrors the tube keys of `charges_for` in
    `apps/website/map-engine/src/data/scenario/ballistics/mortar_fire_solution.rs`
    (`the_offered_weapons_are_the_keys_the_api_accepts` pins the copy);
  - the grid string round-trips every coordinate the inputs accept
    (`every_coordinate_the_inputs_accept_round_trips_through_the_grid_string`), and `restore` keeps
    its grid fallback for rows without numeric coordinates
    (`a_row_saved_before_the_migration_still_restores_and_shows_no_tof_or_charge`);
  - hydration acts only on a batch fetched for the selected event, at most once per event
    (`hydration_refuses_a_batch_fetched_for_a_different_operation`,
    `returning_to_an_operation_does_not_re_hydrate_over_unsaved_edits`);
  - both preview markers move with every coordinate
    (`both_preview_markers_move_when_any_input_moves`), and a missing charge or flight time shows
    "—", never a zero.

## Related documentation

- [Mortar calculator page](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the page's behaviour and design.
- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the fire-mission routes this
  page calls.
