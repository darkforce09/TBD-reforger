# Mortar calculator page

The `/tools/mortar` page: pick a weapon, shell and charge from a ballistics catalog, place a
battery of one to twelve guns (the contract's cap on a saved battery) and a target by grid
reference, give the wind and, for a time-fuzed shell, the
burst height, and the page solves every gun's firing solution on the device with the map engine's
solver. The page is open to every viewer; a signed-in viewer also sees the
[event](/documentation/glossary/a_to_f.md#event) picker and the fire missions saved against the
selected event.

## Contents

```text
apps/frontend/src/v2/pages/field_tools/mortar/
├── catalog_source.rs   the public catalog reads (server or saved copy), the newest version per catalog, the dated offline wording
├── inputs/             weapon and shell, positions, battery, wind and illumination inputs
├── map_picker/         the Everon map: placement picker, click and drag placing, the fire-mission overlay, the mount, the lead gun's crest profile
├── mod.rs              the module tree; re-exports `MortarCalculatorPage`
├── offline_status.rs   the offline pack line: state and download progress, the icon font notice when `data-offline-optional` is `missing`, the "refresh failed, using the saved copy from <date>" notice
├── page.rs             `MortarCalculatorPage`: signals, catalog loading, Calculate, the layout
├── saved_fires/        the save area: event picker, save request, saved list, restore, hydration
├── solution/           the solution panel: battery summary, charge tables, dispersion, fuze, crest
├── solve_bridge.rs     drafts → `FireMissionInputs` → the map engine's `solve_fire_mission`; rows worded by its `solution_wording`
└── tests/              unit tests: inputs and catalog source, solve bridge, map picker, solution, saved fire missions, offline line, shared test mission and catalog
```

## How it works

```text
GET /api/v1/ballistics-catalogs ──▶ newest version per catalog ──▶ chosen CatalogKey
GET /api/v1/ballistics-catalogs/{id}/versions/{v} ──▶ BallisticsCatalog (identity checked)
      (offline: the service worker answers both from its cache)
inputs (weapon, shell, charge, terrain, target, guns, wind, burst height)
      ▲ map clicks and marker drags write 10-figure grid references
      │ Calculate
      ▼
map_picker::profile::lead_gun_profile ──▶ TerrainProfile (lead gun → target, 2 m steps)
solve_bridge::solve_mission ──▶ every problem at once, or
      resolve grids + heights ──▶ FireMissionInputs ──▶ solve_fire_mission ──▶ FireMissionSolution
      ▼
solution panel (battery, crest, fuze, dispersion, charge tables) and the map overlay
      │ Save Fire Mission (signed in, event chosen)
      ▼
POST /api/v1/fire-missions { inputs, client_solution } ──▶ the API re-solves and compares
```

The catalogs are read with the public reads, without credentials. The list is reduced to the
newest version of each catalog; a picker appears when more than one catalog is published. A
document whose `catalog_id` or `catalog_version` differs from the version asked for is refused.
When the server cannot answer (unreachable, or a `5xx` such as a proxy's `502` while the API is
down) the saved copy the offline pack stored answers instead — through the service worker, or read
from Cache Storage by the page when no worker controls it, under the key the pack writes — and the
page says "Offline copy from <date>", the copy's `Date`; a `4xx` is shown as the failure it is. A
read that fails with no saved copy is explained from the offline pack's state (`offline_status`).
The offline pack line under the header shows that state and the download progress on every visit,
adds the icon font notice when the optional icon font is not cached (`data-offline-optional`
`missing`), and, when a re-run could not refresh the pack but every essential file is still cached
(`data-offline-refresh` `kept-saved-copy`), stays ready with "Refresh failed, using the saved copy
from <date>".

Weapons are labelled with their mils convention (6,400 or 6,000). Only the shells the weapon
lists and the catalog defines are offered, and every charge of the shell after "Recommended".
Changing the weapon or the catalog reconciles the selection.

Positions are typed as 6-, 8- or 10-figure grid references or placed on the map. On Everon the
map (`core::map_view::mount::mount_map_view`, terrain-and-imagery scope) shares the page's
`TerrainHeights`, so a terrain height resolves once the 2 m raster loads; a click writes the
position chosen in "Place on the map" as a 10-figure grid reference, and a press on a placed
marker grabs it and drags it (the press never reaches the map's pan). The overlay draws the guns,
the target, the gun→target lines and, after a solve, each gun's dispersion ellipse through the
map engine's `overlay::fire_mission_marks` lanes. Arland has no elevation model: no map, and every
height is typed. A terrain height that is not there yet is an error, never a zero.

Calculate samples the ground from the lead gun to the target (`spatial::los::terrain::sampler`)
into a `TerrainProfile`, then `solve_fire_mission` — the one assembler the API re-solves a saved
fire mission with — returns the whole solution. The panel shows a battery line per gun (the laid
charge's elevation, aim azimuth and time of flight), the crest check (a warning when the flight
passes below the terrain, or "No crest check" without map heights), the time fuze for a burst
height, the lead gun's dispersion labelled "Interpretation, not verified in-engine", and one
charge table per gun in the weapon's mils and in degrees.

The save area renders inside `AuthGate` while the platform answers (while the catalog comes from
the offline copy it is replaced by a needs-a-connection notice, with no sign-in link): the event picker, "Save Fire Mission" (enabled with a
solution and an event), and the saved fire missions of the selected event. The save posts a
`FireMissionSave` whose `client_solution` is the page's own solution; a 422 `solution_mismatch`
says the server's solve differs. The newest saved row fills the drafts once per event, any row
on a click: every gun with its label, heights (a manual height stays manual), the weapon, shell
and charge, the wind and the burst height; a row stored before catalogs restores its target and
one gun from its coordinates or its legacy `x, y` grid text. The selected event is kept in
`localStorage` under `tbd-mortar-event`.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/tools/mortar` | `MortarCalculatorPage` | route tier `none`; the calculator renders for every viewer, the save area for a signed-in one | full-bleed inside the navigation frame; breadcrumb Field Tools / Mortar Calculator |

## Data

- `GET /api/v1/ballistics-catalogs`: `BallisticsCatalogList`, public.
- `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}`: the `BallisticsCatalog`
  document, public.
- `GET /api/v1/events`: read as `Paginated<EventOption>` for the event picker (signed in).
- `GET /api/v1/events/{id}/fire-missions`: the selected event's saved rows, read as
  `DataEnvelope<SavedFire>` (signed in).
- `POST /api/v1/fire-missions`: a `FireMissionSave` body, answered by `SavedFireMissionAnswer`
  (signed in).
- `/map-assets/everon/…`: the manifest, elevation, hillshade, satellite and grid the map boots.
- `localStorage` key `tbd-mortar-event`.
- The page reads the `AuthStore`, `offline_status()` and, for terrain heights, a
  `TerrainHeights` reader a mounted map view fills. Requests run in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| catalogs loading | the inputs with empty pickers; "Calculate Solution" disabled |
| catalog failure | "No ballistics catalog has been published yet…", or "The ballistics catalogs could not be loaded (…)." / "The chosen ballistics catalog could not be loaded (…)." followed by the offline explanation |
| offline copy | "Offline — solving with the catalog saved on this device." |
| no outcome | "Enter the target and the guns, then calculate." |
| input problems | a list, one sentence per problem |
| solution | the battery table, the crest line, the fuze card, the dispersion card, and one table per gun headed "<label> — <m> m · line <mils> mils · <deg>° · Δh <m> m" |
| map | "Loading the map…", then the map; on a failed mount "The map could not load (…); type the grid references instead."; on Arland a note instead of the map |
| saving | "Saving…", then "Saved (<created_at>)." or the refusal sentence |
| signed out | the save area shows "Sign in to load live data from the platform." and a sign-in link |
| save area | the event picker ("— none (not saved) —" or "<name> — <date>"), "Save Fire Mission" and "Saved Fire Missions" |

## Boundaries

- Depends on: `crate::v2::core::api` (`public_reads::public_get`, `api_get`, the ballistics
  catalog and fire-mission DTOs in `apps/frontend/src/v2/core/api/dto/`),
  `crate::v2::core::offline` (`offline_status`), `crate::v2::core::map_view` (`mount`,
  `handles`, `navigation_math`, `terrain_height`, `terrain_preferences`, `engine_mount`),
  `crate::v2::core::ui` (`AuthGate`, `PageHeader`), `crate::v2::core::utils::datefmt`;
  `map_engine` (`camera::grid_reference`, `data::scenario::ballistics::fire_mission`,
  `battery`, `solver`, `dispersion`, `fuze` and `crest_clearance`, `overlay::fire_mission_marks`,
  `overlay::symbology::markers`, `spatial::los::terrain::sampler`,
  `editing::tools::line_of_sight::terrain_survey::everon_manifest`).
- Used by: the `/tools/mortar` route in `apps/frontend/src/app_routes.rs` and
  `apps/frontend/src/router.rs`; the sidebar's "Mortar Calculator" link in
  `apps/frontend/src/v2/pages/navigation/nav_config.rs`; the offline pack trigger in
  `apps/frontend/src/v2/core/offline/offline_pack.rs`; the DOM oracle's `mortar` capture
  in `tools/developer_tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules:
  - the page's solution is the engine's `solve_fire_mission` answer byte for byte
    (`the_solution_is_the_engine_fire_mission_solution_byte_for_byte`), from inputs pinned to the
    catalog (`the_drafts_map_onto_the_engine_inputs_pinned_to_the_catalog`);
  - every gun is solved as if it fired alone (`every_gun_is_solved_as_if_it_fired_alone`);
  - 6-, 8- and 10-figure grids resolve to their cell centres
    (`grids_of_six_eight_and_ten_figures_resolve_to_their_cell_centres`), and Arland never reads
    a terrain height (`arland_only_resolves_manual_heights_and_never_reads_the_terrain`);
  - every input problem is reported at once (`every_input_problem_is_reported_at_once`);
  - hydration acts only on a batch fetched for the selected event, at most once per event
    (`hydration_refuses_a_batch_fetched_for_a_different_event`);
  - the save posts the solve's own inputs and solution
    (`the_save_body_carries_the_solved_inputs_and_the_client_solution`);
  - a click writes only the placed position (`a_click_writes_only_the_placed_position_as_a_ten_figure_grid`),
    and the crest check warns over a ridge (`the_crest_check_clears_low_ground_and_warns_over_a_ridge`);
  - the dispersion is labelled an interpretation
    (`the_dispersion_is_labelled_an_interpretation_not_verified_in_engine`).

## Related documentation

- [Mortar calculator page](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the page's behaviour and design.
- [Firing solver](/legacy/map_engine/src/data/scenario/ballistics/solver/README.md) — the
  solver the page runs.
- [Offline core](/apps/frontend/src/v2/core/offline/README.md) — the offline pack and
  its state.
- [Operations domain](/apps/api/src/operations/README.md) — the catalog and
  fire-mission routes.
