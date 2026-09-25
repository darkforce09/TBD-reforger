**Status:** live

# Building viewer bench

The `/debug/building-viewer` bench: one building blueprint, extracted from the game by the
[Workbench](/documentation_v2/glossary.md#workbench) exporter, drawn as a floor plan, with a
draggable observer and target whose line of sight is traced through the building's occlusion mesh,
and a per-floor viewshed wash. It exists so a developer can check what the extractor produced for
a building, and how the line-of-sight code reads it, before the data reaches the
[Mission Creator](/documentation_v2/glossary.md#mission-creator).

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/debug/building_viewer/`](/apps/website/frontend/src/v2/apps/debug/building_viewer/)
  and its module root `apps/website/frontend/src/v2/apps/debug/building_viewer.rs`: `page.rs`
  holds the route component `BuildingViewerPage`, the signals and the verdict panel; `geom.rs` the
  pure geometry; `live.rs` and `live/` the browser host. The interior lanes it draws on live in
  `apps/website/frontend/src/v2/apps/debug/building_interior.rs`. The folder's
  [README](/apps/website/frontend/src/v2/apps/debug/building_viewer/README.md) describes each file.
- Entry: the route, its URL flags and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/apps/debug/building_viewer/README.md#routes). No
  navigation entry links it; it is opened by URL.
- Related: the [world line-of-sight bench](/documentation_v2/website/frontend/apps/debug/world_los_page.md),
  which probes the same kind of occluder across a whole map area; the blueprint contract
  `contracts_v2/definitions/building-blueprint.schema.json`.

## Behaviour

The texts of every loading, failure and verdict state are in the README's
[States](/apps/website/frontend/src/v2/apps/debug/building_viewer/README.md#states). The bench
needs no sign-in.

### Loading a building

1. The bench loads the blueprint named by `?prefab=`, by default the wooden farmhouse
   `/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.json`, then its `.bvh` occlusion
   sidecar and its `<slug>.instances.json`, and under `?scene=1` the exterior trees of
   `<slug>.scene.json`. The same folder, `assets_v2/terrains/everon/prefabs/buildings/`, holds
   the hand-authored `FarmHouse_E_1L01.json`, which has no sidecar beside it and so exercises the
   blueprint fallback.
2. With the instances it assembles the furnished building (shell, door leaves, frames, panes and
   furniture) and runs line of sight, the wash and the section cuts over that; without them it
   stays a shell and says so. Without the sidecar the plan still draws from the blueprint, and line
   of sight, the wash and the mesh drawing stay off.
3. The building sits at a fixed world anchor with game north up, and the camera fits it on the
   first load. Every parameter of a run is in the URL (`?doors=open`, `?a=x,y,z`, `?b=x,y,z`,
   `?force=webgl` besides the two above), so a reading reproduces exactly.

### Reading the plan

1. The drawing is the mesh's own: for the viewed floor, section cuts through the collision
   triangles at eye height (walls as double outlines, windows as gaps, furniture as outlines), a
   dim low cut for sills, the slab faces as the floor, and the floors below through any void. The
   blueprint's apertures, furniture, stairs, door swings and rings lie over it as annotations.
2. A click on the building opens the floor rail, a vertical stack from the ground floor up with
   "Roof" on top, which stays open once opened. The Roof view shows the roof surface from eave to
   ridge and a chip with the roof type and heights.
3. Past about 16 px per metre, each window of the viewed floor shows a badge with its sill height
   and window height.
4. Drag on the canvas pans, the wheel zooms up to 64 px per metre, and a click on a door leaf
   swings it, which re-runs the cuts and line of sight.

### Probing line of sight

1. The observer A and the target B are draggable markers; "Observer Y" and "Target Y" sliders set
   their heights from 0 to 10 m.
2. Every move re-runs the trace. The verdict panel reads "CLEAR" or "BLOCKED" with the
   concealment and what the ray crossed: glass, doors, canopy, cover, or the blocker, including
   "blocked by roof @ <y> m" when the ray pierces the roof.
3. The ray is coloured along its path: clear through open doors and apertures, glass through
   windows, foliage through canopy, cover through furniture, blocked from a wall, roof, solid,
   frame or prop on, and a window, stair or piece of furniture with full concealment blocks too.
4. Alt+click moves A and fills its viewshed: a wash of the cells A sees, cast at eye height on
   every level every 0.25 m, of which the floor rail shows the viewed level's; the Roof view has no
   wash. "✕ clear" turns it off.

## Data

The README's [Data](/apps/website/frontend/src/v2/apps/debug/building_viewer/README.md#data)
lists every file the bench fetches. They are static files under `/map-assets`, which the
[API](/documentation_v2/glossary.md#api) serves and Trunk proxies in development; the bench calls
no `/api/v1` route and writes nothing. The blueprint follows
`contracts_v2/definitions/building-blueprint.schema.json`; the sidecar and the instances come
from the same export.

## Design

- Chromeless and full-bleed: the canvas fills the window under a header line and a floating panel
  with the sliders, the hint line, the verdict and the legend.
- A debugging instrument, not a product page: no visual reference set exists, and the design
  follows the lane colours in `geom.rs` and the legend drawn on screen.

## Open work

- [T-946.71 — Viewshed wash lane shipped with no consumer](/.ai/tickets/T-946.71.toml) (idea, no
  plan): the bench computes each wash in one synchronous call; the budgeted wash lane the map
  engine offers has no caller yet.

## Decisions

- The bench is a URL-only route with no sign-in and no navigation entry: it is a developer's
  instrument, and every run reproduces from its address.
- It reads committed assets and engine code only, and shares none of the Mission Creator's boot
  (no local draft, no terrain or satellite layers), so a reading depends on the building alone.
- The plan is drawn from the collision mesh, with the blueprint as annotation and fallback: what
  blocks sight is the mesh, so the drawing shows what the trace sees.
- All geometry is pure and tested natively; the browser half only wires signals, listeners and
  the engine.
