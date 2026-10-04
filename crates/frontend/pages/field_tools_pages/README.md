# Field tools pages

The `field_tools_pages` crate: the Field Tools section of the single-page app's sidebar, the
standalone tactical aids — today the mortar calculator at `/tools/mortar`, which solves a
battery's firing solution on the device from a published ballistics catalog, places the guns
and the target on the Everon map, saves and restores fire missions against an
[event](/documentation/glossary/a_to_f.md#event), and keeps working offline from the offline
pack.

## Contents

```text
crates/frontend/pages/field_tools_pages/
├── Cargo.toml  the package: `frontend_api_dtos`, `frontend_offline`, the ballistics, grid, overlay and line-of-sight crates, `leptos`; on `wasm32` the transport, session, UI and map view crates; layout tier 10
└── src/        the mortar calculator: catalog source, inputs, solve bridge, solution panel, map picker, saved fire missions, offline line, their tests
```

## How it works

The app's route table (`apps/frontend/src/app_routes.rs`) mounts `MortarCalculatorPage` at
`/tools/mortar`, open to every viewer. The page reads the public ballistics catalogs, or the copy
the offline pack saved, and solves every gun with `fire_mission_planning`'s
`solve_fire_mission`, the assembler the [API](/documentation/glossary/a_to_f.md#api) re-solves a
saved fire mission with. A signed-in viewer also picks an event and saves the solve against it;
the API refuses a save whose solution differs from its own. The
[source tree README](src/README.md) walks through the crate, and the
[mortar README](src/mortar/README.md) through each file of the page.

The route component and every panel that fetches or mounts the map are compiled for `wasm32`
only, because the endpoints and the map engine exist only in the browser build. The catalog
choice, the input parses, the solve bridge, the map marks and terrain profile, and the saved
fire missions' restore and save body compile on every target, so their tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p field_tools_pages   # the inputs, the catalog source, the solve, the map picker, the saved fire missions
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `mortar::MortarCalculatorPage` (`wasm32`): the route component, also at `mortar::page` and in
  `prelude`.
- `mortar::saved_fires::connection_gate::MortarSaveSection` and
  `mortar::saved_fires::save_area::MortarSaveArea` (`wasm32`), with the types their props name:
  public because the props builder `#[component]` derives has `pub` methods; the page mounts the
  section and the section mounts the area, their only callers. The
  [source tree README](src/README.md) lists them.
- `prelude`: `MortarCalculatorPage`.

## Boundaries

- Depends on: `frontend_api_dtos` (the catalog list and the saved fire mission wire types),
  `frontend_offline` (the offline pack's status and saved copies), `ballistics_model`,
  `ballistics_solver`, `fire_mission_planning`, `map_coordinates`, `overlay_instances`,
  `terrain_line_of_sight`, `terrain_elevation`, `map_editing_tools`, `leptos`, `serde`,
  `serde_json`; on `wasm32`, `frontend_transport`, `frontend_session`, `frontend_ui`,
  `frontend_map_view`, `map_draw_lanes`, `unit_symbology`, `wasm-bindgen` and `web-sys`;
  `frontend_test_support` and `offline_cache_policy` for its tests only.
- Used by: the single-page app (`apps/frontend`), whose route table mounts the page; the API test
  `apps/api/tests/fire_mission_solution.rs`, which reads `src/mortar/saved_fires/restore.rs` as
  text to pin its copy of the legacy grid reader.
- Rules: a page crate depends on foundation and feature crates only, never on another page crate,
  a workspace or the app (`cargo xtask ci verify-workspace-laws`); the page solves with the
  ballistics crates' solver and never with a copy of it
  (`the_solution_is_the_engine_fire_mission_solution_byte_for_byte` in
  `src/mortar/tests/solve_bridge.rs`).

## Related documentation

- [Mortar calculator page](/documentation/crates/frontend/pages/field_tools_pages/mortar/mortar_calculator_page.md)
  — the page's behaviour and design.
- [Offline mortar page runbook](/documentation/runbooks/offline_mortar_page.md) — checking the
  page offline.
- [Frontend page crates](/crates/frontend/pages/README.md) — the layer this crate sits in.
