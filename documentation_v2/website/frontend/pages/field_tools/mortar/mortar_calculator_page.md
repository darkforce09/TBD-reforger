**Status:** live

# Mortar calculator page

The `/tools/mortar` page, titled "Mortar Calculator": a signed-in member enters a firing position,
a target and a mortar tube, and the [API](/documentation_v2/glossary/a_to_f.md#api) returns the firing
solution. With an [event](/documentation_v2/glossary/a_to_f.md#event) selected, the solution is saved
against it, so the gun line can reload that event's fire missions.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/field_tools/mortar/`](/apps/website/frontend/src/v2/pages/field_tools/mortar/):
  `page.rs` holds the route component `MortarCalculatorPage`, both fetches and the solve request;
  `map_picker.rs` the four coordinate fields and the preview; `weapon_selector.rs` the tube and
  event pickers; `firing_solution.rs` the solution card; `saved_fires.rs` the saved list and the
  restore rules. The folder's
  [README](/apps/website/frontend/src/v2/pages/field_tools/mortar/README.md) describes each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/field_tools/mortar/README.md#routes).
- Related: the [operations](/documentation_v2/glossary/n_to_z.md#operations) domain's fire-mission
  routes, in the [operations domain README](/apps/website/api_v2/src/operations/README.md); the
  solver `solve_fire_mission` in
  `apps/website/map-engine/src/data/scenario/ballistics/mortar_fire_solution.rs`, which the API
  calls.

## Behaviour

The page body sits in `AuthGate`; the session texts and every label and toast are in the README's
[States](/apps/website/frontend/src/v2/pages/field_tools/mortar/README.md#states).

### Solving

1. The operator picks a tube ("M252 81mm", "M821 81mm", "2B14 82mm" or "M120 120mm") and types the
   firing position ("FP X", "FP Y") and the target ("TGT X", "TGT Y") as metric map coordinates,
   x east and y north. They are plain metres, not a military grid reference.
2. The preview draws the two points and the line between them with no terrain behind, framed
   square, centred between them and 1.6 times their larger span, so both markers stay on screen at
   any separation; it moves with every keystroke.
3. With no event picked, "Calculate Solution" asks the API to solve only. The card fills and reads
   "Not saved — lost on reload", and the toast says to pick an operation to keep it.
4. With an event picked, "Calculate & Save" solves and saves in one request, so the card never
   shows numbers that failed to save. It reads "Saved — survives a reload", the toast says so, and
   the saved list refetches.
5. The card shows the distance in metres, the azimuth in degrees, the elevation in mils, the charge
   and the time of flight in seconds. A refusal toasts the API's sentence, for example an unknown
   tube or a target out of range, or "Could not compute firing solution".

### Saving to an event

1. The "Operation" picker lists the upcoming events the viewer may see, as "<name> — <date>"
   ("Untitled Operation" without a name), after "— none (not saved) —".
2. The picked event is kept in the browser's `localStorage` under `tbd-mortar-event`, so a reload
   returns to it. Once the list arrives, a stored id that names no listed event is cleared, since
   the API would save against it without complaint and nobody could find the row again.
3. The "Saved Fire Missions" card lists the picked event's saved rows, newest first, as "<fp grid>
   → <target grid>" over "<n> m · <deg>° · <n> mils". A click loads a row back into the form.
4. The newest saved row fills the form once per event, on the first batch fetched for it; a batch
   fetched for another event is never acted on, and returning to an event does not overwrite
   edits in progress.
5. Each row stores its coordinates twice: as numbers and as the text `x, y`. Restoring reads the
   numbers first and falls back to the text; a row with neither restores nothing, and a charge or
   flight time a row lacks shows "—", never zero.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/field_tools/mortar/README.md#data) lists
each call with its DTO. Server-side:

- `POST /api/v1/fire-missions/solve` (`solve_fire` in
  `apps/website/api_v2/src/operations/handlers/fire_missions.rs`): any signed-in member. All five
  fields must be present, since `0` is a real coordinate. The tube is a lookup key: an unknown,
  misspelled or padded tube answers 400 before any range check, and is never replaced by another
  tube's table. The solver takes the flat distance and grid azimuth, then the high-angle solution
  of the lowest charge that reaches, from each tube's muzzle-velocity table (five charges for the
  81mm and 82mm tubes, four for the 120mm), with no air drag, height difference or wind. A target
  no charge reaches answers 422. The reply holds the distance, azimuth in degrees and mils,
  elevation in mils, charge and time of flight.
- `POST /api/v1/fire-missions` (`save_fire`, same file): the same solve, then one row stores the
  tube, both grid texts (trimmed, required), the four coordinates and the seven solution values,
  with the caller as author; it answers 201 with the solution and the row. `event_id` may be left
  out, but a blank or malformed one answers 400; the API does not check that the event exists or
  that the caller may see it.
- `GET /api/v1/events/{id}/fire-missions` (`list_event_fire_missions`, same file): the event's rows
  oldest first, which the page reverses; the API does not check the caller's access to the event.
- `GET /api/v1/events` (`list_events` in
  `apps/website/api_v2/src/operations/handlers/event_listing.rs`): the default `upcoming` scope,
  first page, filtered to the events the caller may see, which feeds the picker.

## Design

- A full-bleed page on the topographic background under a header: the inputs panel with the
  pickers and the four fields, and the preview panel, over whose lower corners float the
  saved-missions card (left) and the solution card titled "Firing Solution — <weapon>" (right).
- Design target: the [mortar calculator blueprint](/documentation_v2/website/frontend/pages/field_tools/mortar/visual_references/mortar_calculator_blueprint/README.md),
  a design-phase reference. The built page differs from the blueprint:
  - the points are typed into four number fields; the preview has no map and its markers cannot be
    dragged;
  - the solution adds the charge and the time of flight to distance, azimuth and elevation;
  - the tube and event pickers, the save state line and the saved list are additions.
- An earlier layout sketch drew each position with an altitude and a charge selector (charges 0
  to 2); the built page takes no altitude, and the API picks the charge.

## Open work

- [T-940.10 — Mortar ballistics crate for API and offline frontend](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_10_plan.md)): the solver gains height
  difference, drag, wind, dispersion and battery solutions in a shared crate, and the page solves
  locally too, so it works offline and shows dispersion and battery rows; the API keeps the save.

## Decisions

- The server solves, the page renders: one solver serves the page and the stored rows, so a saved
  fire mission always holds the numbers the operator saw.
- Solving and saving are one request when an event is picked: the card never shows a solution
  that failed to save, and without an event it says plainly that nothing was saved.
- An unknown tube is an error, never a fallback: another tube's table gives a confident but wrong
  elevation, tens of mils off.
- The picked event lives in the browser, and a stale one is dropped when the event list arrives,
  because the API stores a fire mission against any event id without checking it.
