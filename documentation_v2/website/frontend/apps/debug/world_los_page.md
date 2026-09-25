**Status:** live

# World line-of-sight bench

The `/debug/world-los` bench: it loads the committed object catalogue around one point of the
Everon map, draws the placed objects as plan footprints with every nearby building cut at eye
height, and probes one segment from A to B through the world occluder, reporting the verdict, the
hits and how much of the catalogue was loaded. It exists so a developer can check the object
line of sight that the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
line-of-sight tool relies on, over the same loader, without the editor around it.

## Where it lives

- Code: the route component `WorldLosPage` and its defaults in
  `apps/website/frontend/src/v2/apps/debug/world_los.rs`; the browser host in
  [`apps/website/frontend/src/v2/apps/debug/world_los/`](/apps/website/frontend/src/v2/apps/debug/world_los/);
  the pure plan geometry (footprints, section cuts, the probe ray) in
  `apps/website/frontend/src/v2/apps/debug/world_los_scene.rs`. The host folder's
  [README](/apps/website/frontend/src/v2/apps/debug/world_los/README.md) describes each file.
- Entry: the route, its URL flags and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/apps/debug/world_los/README.md#routes). No navigation entry
  links it; it is opened by URL.
- Related: the [building viewer bench](/documentation_v2/website/frontend/apps/debug/building_viewer_page.md),
  which probes one building in detail; the map engine's world occluder in
  `apps/website/map-engine/src/spatial/los/world/`.

## Behaviour

The texts of every status, failure and probe line are in the README's
[States](/apps/website/frontend/src/v2/apps/debug/world_los/README.md#states). The bench needs no
sign-in.

### Loading an area

1. The run reads its parameters from the URL: the centre `?x=` and `&y=` in map metres (by
   default 9363, 285, the farmhouse village the building lanes were built against), the radius
   `&r=` (150 m, clamped to 20 to 600), the ray ends `&a=` and `&b=` in the engine frame (east, up,
   north), the cut height `&eye=` (1.8 m) and `&force=webgl`.
2. It reads the Everon asset manifest and prefab table, ingests every object chunk the radius
   covers, then runs up to 64 loader passes that expand the object descriptors and fetch the
   collision geometry (BLAS) those chunks place, reporting each pass on the status line.
3. The mean elevation of the object rows inside the radius stands in for the ground, since the
   bench loads no terrain; the cut plane and the default ray height sit `eye` metres above it.
4. The plan draws each placed object inside the radius as a footprint: buildings as slabs, props
   grey, trees green, rocks brown, and an amber outline on an object whose geometry is still
   loading. Up to 96 upright buildings are also cut at eye height, drawn as a white strip.

### Probing a segment

1. Without `&a=` and `&b=`, A and B sit 40 m either side of the centre at eye height. A click
   (without moving) sets A, the next click sets B; drag pans and the wheel zooms.
2. Each probe traces A to B through the world occluder and writes the verdict line: both ends, the
   length, the verdict, the concealment and the time taken. The verdict is clear, blocked, or
   provisional when a still-loading proxy or a chunk that is not resident decided it.
3. The coverage line counts the chunks the segment crossed and names the ones missing, the proxy
   prefabs it met and the collision meshes still pending; the list names the first 16 hits by the
   instance each one struck.
4. The ray is drawn coloured by what it crossed: green clear, cyan glass, yellow-green canopy, red
   blocked and amber provisional.

## Data

The README's [Data](/apps/website/frontend/src/v2/apps/debug/world_los/README.md#data) lists
every file the bench fetches. They are static files under `/map-assets/everon/`, which the
[API](/documentation_v2/glossary.md#api) serves and Trunk proxies in development: the manifest,
the prefab table, the object chunks and the descriptors and BLAS files the loader reads. The bench
calls no `/api/v1` route and writes nothing.

## Design

- Chromeless and full-bleed: the canvas fills the window, and one floating panel holds the title,
  the status, verdict and coverage lines, the hit list, the stats, the legend and the flag hint.
- A debugging instrument, not a product page: no visual reference set exists.

## Open work

- [T-1039 — Fix world line-of-sight bench frame pump running after unmount](/.ai/tickets/T-1039.toml)
  (idea, no plan): leaving the bench stops its render loop and removes its window listeners, as
  the building viewer does.
- [T-1042 — Rename ticket ids out of code names and UI strings](/.ai/tickets/T-1042.toml) (idea,
  no plan): the panel title stops showing a ticket id.
- [T-090.12.7 — Docs pass: LOS tool, world-los bench, map-assets, MCP](/documentation_v2/tickets/specs/t090_091_map_terrain_program.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-090_12_7_plan.md)): the documentation pass that
  names this page, the frontend route table rows and the map-assets rows.

## Decisions

- The bench loads objects through the same loader the Mission Creator's line-of-sight tool
  uses, so what it shows is what the tool sees.
- Every run is in the URL, with no sign-in and no navigation entry: a reading reproduces from its
  address, and the bench never touches a mission.
- It loads no terrain: the ground is the mean object-row elevation inside the radius, which keeps
  the bench about objects alone.
- An unresolved answer is labelled provisional rather than guessed clear or blocked.
