**Status:** live

# Mortar calculator page

The `/tools/mortar` page, titled "Mortar Calculator": anyone picks a weapon, shell and charge from
a ballistics catalog, places one to twelve guns and a target on the Everon map or by grid
reference, gives the wind and a burst height, and the page solves every gun's firing solution on
the device with the ballistics crates. It works offline once visited. A signed-in member can
save the fire mission against an [event](/documentation/glossary/a_to_f.md#event), so the gun
line can reload it.

## Where it lives

- Code: [`apps/frontend/src/pages/field_tools/mortar/`](/apps/frontend/src/pages/field_tools/mortar/):
  `page.rs` holds the route component `MortarCalculatorPage`; `catalog_source.rs` the public
  catalog reads; `inputs/` the weapon and shell, position, battery, wind and illumination inputs;
  `map_picker/` the Everon map with placing, dragging, the fire-mission overlay and the crest
  profile; `solve_bridge.rs` the mapping onto `fire_mission_planning`'s solver; `solution/` the solution
  panel; `saved_fires/` the save area; `offline_status.rs` the offline pack line. The folder's
  [README](/apps/frontend/src/pages/field_tools/mortar/README.md) describes each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/frontend/src/pages/field_tools/mortar/README.md#routes).
- Related:
  - the [game ballistics engine](/documentation/crates/ballistics/game_ballistics_engine.md),
    whose `solve_fire_mission` the page and the API both run;
  - the [ballistics catalogs page](/documentation/apps/frontend/pages/administration/ballistics_catalogs/ballistics_catalogs_page.md),
    where administrators publish the catalogs the page solves with;
  - the [offline core](/apps/frontend/src/foundation/offline/README.md) and the
    [offline mortar page runbook](/documentation/runbooks/offline_mortar_page.md);
  - the [game ballistics design note](/documentation/apps/api/verification_evidence/game_ballistics.md),
    which records the model, the tolerances and the operator decisions.

## Behaviour

The page renders for every viewer; only the save area sits in `AuthGate`. Every label, message and
state text is in the README's
[States](/apps/frontend/src/pages/field_tools/mortar/README.md#states).

### Choosing the catalog, weapon, shell and charge

1. The page reads the published catalogs without credentials and keeps the newest version of each;
   a picker appears only when more than one catalog is published. A document whose id or version
   differs from the one asked for is refused, so a solution always names the catalog it came from.
2. Weapons are labelled with their mils convention (6,400 for the M252, 6,000 for the 2B14). Only
   the shells the weapon lists and the catalog defines are offered, and the charge picker lists
   "Recommended (fewest rings that solve)" and then every charge of the shell. Changing the weapon
   or the catalog reconciles the selection.
3. With no catalog published the page says so; there is no built-in fallback table, because every
   catalog, vanilla included, is uploaded and calibrated through the API.

### Placing the guns and the target

1. Each position is a 6-, 8- or 10-figure grid reference, resolved to the centre of its cell, with
   a height from the terrain or typed by hand. The battery holds one to twelve guns, each with its
   label.
2. On Everon the map mounts through the Mission Creator's shared map seam with terrain and imagery
   only. "Place on the map" chooses which position a click writes, as a 10-figure grid; a press on a
   placed marker drags it instead of panning the map. The overlay draws the guns, the target, the
   gun-to-target lines and, after a solve, each gun's dispersion ellipse.
3. Terrain heights come from the full 2 m elevation raster once it loads; a height that has not
   loaded is an error, never zero. Arland has no elevation model, so it has no map and every height
   is typed.

### Solving

1. "Calculate Solution" reports every input problem at once, as a list. Otherwise it samples the
   ground from the lead gun to the target into a terrain profile and calls the map engine's
   `solve_fire_mission` with the drafts pinned to the catalog.
2. The panel shows a battery line per gun (the laid charge's elevation, aim azimuth and time of
   flight), the crest check (a warning when the flight passes below the terrain, "No crest check"
   without map heights), the time fuze for a burst height on a time-fuzed shell, the lead gun's
   dispersion labelled "Interpretation, not verified in-engine", and one table per gun with every
   charge in the weapon's mils and in degrees, or that charge's refusal.
3. Under wind the aim azimuth differs from the gun-to-target bearing: the solver aims upwind so the
   drift carries the shell onto the target, and each row shows its deflection and range
   corrections.
4. Every gun is solved as if it fired alone; there is no sheaf, time on target or adjust-fire.

### Working offline

1. On the first visit the offline core downloads the offline pack: the app shell, every published
   catalog version, the Everon manifest, elevation, imagery and map tiles z0–6 (about 248 MB),
   after checking the storage estimate and asking for persistent storage. The line under the
   header shows the state and the progress on every visit.
2. Once the pack is ready the page, its catalogs and the map load and solve with no connection; a
   catalog read answered from the offline copy says "Offline — solving with the catalog saved on
   this device.", and a failed read is explained from the pack's state.
3. Saving, the event list and the saved fire missions need the API; the offline pack holds none
   of them. While the catalog comes from the offline copy the save area says "Saving fire missions
   and the saved fire missions need a connection to the platform; the calculator above keeps
   working offline." and shows no sign-in link.

### Saving to an event

1. Signed in, the save area shows the event picker (the upcoming events the viewer may see),
   "Save Fire Mission" (enabled with a solution and an event) and the event's saved fire missions.
   Signed out, it shows "Sign in to load live data from the platform." and a sign-in link.
2. The save posts the solve's own inputs and the page's solution. The API re-solves them with the
   same code; a difference beyond 1 weapon mil or 0.1 s is refused as `solution_mismatch`, so a
   stored fire mission always holds the numbers the operator saw.
3. The picked event is kept in `localStorage` under `tbd-mortar-event`.
4. The newest saved row fills the form once per event, on the first batch fetched for it, and any
   row fills it on a click: every gun with its label and heights (a manual height stays manual),
   the weapon, shell and charge, the wind and the burst height. A row saved before catalogs
   restores its target and one gun from its coordinates or its legacy `x, y` grid text and cannot
   be re-solved.

## Data

The README's [Data](/apps/frontend/src/pages/field_tools/mortar/README.md#data) lists
each call with its DTO. Server-side:

- `GET /api/v1/ballistics-catalogs` and `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}`
  (`apps/api/src/operations/handlers/ballistics_catalogs/reads.rs`): public. The list
  holds one summary per stored version; a version document is immutable, answered with its sha256
  as `ETag` and `Cache-Control: public, max-age=31536000, immutable`, so the offline copy never
  goes stale.
- `POST /api/v1/fire-missions` (`apps/api/src/operations/handlers/fire_missions/save.rs`):
  any signed-in member. It loads the pinned catalog (404 when unknown), re-solves through
  `solve_fire_mission`, compares with the client solution (422 `solution_mismatch` with both
  values, 422 `fire_mission_refused` when the inputs do not solve, 422 `no_firing_solution` when
  the lead gun's fired charge does not solve), then stores the server solution and the guns in one
  transaction and answers 201 with the solution and the row. An unknown event answers 404.
- `GET /api/v1/events/{id}/fire-missions` (`apps/api/src/operations/handlers/fire_missions/list.rs`):
  the event's rows with their guns, behind the event's viewer access (404 or 403 as that check
  decides). Rows saved before catalogs list with their original weapon strings, "M120 120mm"
  included.
- `GET /api/v1/events` (`list_events` in
  `apps/api/src/operations/handlers/event_listing.rs`): the default `upcoming` scope,
  first page, filtered to the events the caller may see, which feeds the picker.
- `/map-assets/everon/…`: the manifest, elevation, hillshade, imagery and grid the map boots, and
  `tiles/map/index.json`, the tile list of the offline pack.

## Design

- A full-bleed page on the topographic background under the header and the offline pack line:
  the inputs column (catalog, weapon, shell, charge, terrain, target, battery, wind, burst height,
  "Calculate Solution"), the map panel with its placement picker, the solution panel, and the save
  area.
- Design target: the [mortar calculator blueprint](/documentation/apps/frontend/pages/field_tools/mortar/visual_references/mortar_calculator_blueprint/README.md),
  a design-phase reference. The built page follows its map with markers placed and dragged on it.
  It differs:
  - positions are grid references with heights, and a battery of guns shares the target;
  - the weapon, shell, charge, wind and burst height are inputs;
  - the solution is a panel of battery rows, charge tables, crest, fuze and dispersion instead of
    three readouts of distance, azimuth and elevation;
  - the event picker, the saved fire missions and the offline pack line are additions.

## Open work

- [T-940.10 — Mortar ballistics crate for API and offline frontend](/documentation/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation/tickets/plans/t-940_10_plan.md)): built by milestone B as this
  page's on-device solve and offline pack; the registry closes it with the milestone.
- [T-1177 — Check event existence and access for fire mission save, list](/.ai/tickets/T-1177.toml)
  (idea): built by milestone B (the event foreign key and the list's viewer access); closed with
  the milestone.
- [T-1245 — Share the mortar save DTO and decode refresh token_type](/.ai/tickets/T-1245.toml)
  (idea): built by milestone B (`SavedFireMissionAnswer`, `token_type` "Bearer"); closed with the
  milestone.
- [T-1045 — Rewrite stale frontend doc comments naming missing code and behaviour](/.ai/tickets/T-1045.toml)
  (idea): this page's comments are current; the ticket covers the rest of the frontend.

## Decisions

- The page solves on the device with the same assembler the API re-solves with: the page works
  without the API and offline, and a saved fire mission is the server's own solution, refused when
  it differs from what the operator saw.
- Physics that matches the game engine, never lookup tables: the flight model is the engine's own
  integration step, calibrated against the game's tables and the engine oracle before a catalog is
  accepted, so modded mortars and artillery arrive as catalogs.
- Heights are inputs to the solver and sampled by the page: the solver stays free of terrain, and
  Arland, with no elevation model, still solves from typed heights.
- The dispersion is labelled an interpretation: no engine call returns a dispersion, so the
  ellipse is derived from the game's parameters and says so.
- A saved row pins its catalog id and version: catalog versions are immutable, so an old fire
  mission re-solves with the numbers it was saved with.
