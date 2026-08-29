# Building Viewer (debug bench)

**Route:** `/debug/building-viewer` (public, URL-only — no sidebar/nav entry; chromeless + full-bleed)
**Live source:** `apps/website/frontend/src/pages/debug/building_viewer.rs`
**Status:** in-progress (Phase A of the building-blueprint extraction program)

## Purpose

Hyper-focused single-prefab test bench for the architectural blueprint pipeline: renders one
building-blueprint JSON (`packages/tbd-schema/schema/building-blueprint.schema.json` contract,
produced by the Workbench extractor in `apps/mod/tbd-export`) and drives the 2.5D
line-of-sight raycaster (`map_engine_core::building_blueprint::evaluate_los`) interactively.
Exists so the operator can visually verify extractor output against the real prefab before the
blueprint layer reaches the mission creator.

## Behavior

- Loads `/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01.json` by default;
  `?prefab=<map-assets path>` overrides.
- Renders on the real wgpu engine (`map_engine_render::RenderEngine`) via the generic vector
  lanes — floor plate, thickness walls, window/door apertures (open/closed), swing arcs,
  furniture cover plates, stairs with tread hatch, ghost centerlines for inactive floors.
- Click the building → floor selector tabs appear (stay visible once opened).
- Draggable observer (A) / target (B) markers + elevation sliders (0–10 m); the LOS ray is
  colored per `LosHit` span: green clear, cyan through glass, yellow past cover, red blocked;
  verdict panel lists traversed apertures / blocker / concealment. Blueprints with a `roof`
  heightfield block rays that pierce the top surface — the verdict then reads
  "blocked by roof @ <y> m".
- Wheel zooms (engine ortho camera, ≤ 64 px/m), drag pans; window sill/height badges appear
  past ~16 px/m.

## Notes

Pure geometry (world mapping, camera fit, tessellation, ray coloring, point-in-polygon) lives in
the page's `geom` module and is native-tested (`cargo test -p website-frontend building_viewer`).
