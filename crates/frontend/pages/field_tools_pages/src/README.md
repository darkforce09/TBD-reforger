# Field tools pages source

The source tree of `field_tools_pages`: the standalone tactical aids, pages that stand on their
own rather than hanging off a [mission](/documentation/glossary/g_to_m.md#mission) or an
[event](/documentation/glossary/a_to_f.md#event). The crate holds one page, the mortar
calculator. The [crate README](../README.md) gives the package, its commands and its boundaries.

## Contents

```text
crates/frontend/pages/field_tools_pages/src/
├── lib.rs      the crate root: the module tree
├── mortar/     the `/tools/mortar` page: catalog weapon, guns and target in, a local firing solution out
└── prelude.rs  the route component the app's route table mounts
```

## How it works

Each page here owns a route under `/tools/`, its own fetches and its own signals; outside the
crate only the route table and the sidebar's "Field Tools" section refer to it, so a page can be
removed without touching the rest of the tree. The mortar calculator reads the public
ballistics catalogs of the [operations](/documentation/glossary/n_to_z.md#operations) domain of the
[API](/documentation/glossary/a_to_f.md#api) (or the offline copy of them), solves the firing
solution on the device with the ballistics crates' solver, and lists the fire missions saved against an
event for a signed-in viewer. The debug benches at `/debug/building-viewer`, `/debug/world-los`
and `/debug/ballistics-agreement` are apps, in `crates/frontend/workspaces/debug_benches/src/`, not field tools.

The route component, the catalog fetch, the map picker's mount and every panel that fetches are
compiled for `wasm32` only, since the endpoints and the map engine they reach exist only in the
browser build. The pure readers under them — the catalog choice, the input drafts and their
parses, the solve bridge, the map picker's marks and terrain profile, the saved fire missions'
restore and save body — are compiled for `wasm32` and for the native test build, where their
tests run.

## Public surface

- `mortar::MortarCalculatorPage` (`wasm32`): the route component `apps/frontend/src/app_routes.rs`
  binds to `/tools/mortar`, also reachable at `mortar::page` and in `prelude`.
- `mortar::saved_fires::connection_gate::MortarSaveSection` and
  `mortar::saved_fires::save_area::MortarSaveArea` (`wasm32`): public only because the props
  builder `#[component]` derives has `pub` methods; the page and the connection gate are their
  one caller each. The types their props name are public for the same reason:
  `catalog_source::CatalogOrigin`, `inputs::battery::GunDraft`, `inputs::positions::PositionDraft`,
  `inputs::weapon_and_shell::ArmamentSelection`, `inputs::wind::WindDraft`,
  `saved_fires::save_area::RestoreTargets`, `solution::SolveOutcome` and
  `solve_bridge::SolvedMission`.

## Boundaries

- Depends on: the foundation crates (`frontend_api_dtos` for the wire types, `frontend_offline`
  for the offline pack's status and saved copies; on `wasm32` also `frontend_transport`,
  `frontend_session`, `frontend_ui` and `frontend_map_view`); `fire_mission_planning`,
  `ballistics_solver` and `ballistics_model` (the ballistics solver); `map_coordinates` (grid
  references); `overlay_instances`, `terrain_line_of_sight`, `terrain_elevation` and
  `map_editing_tools` (the map picker's marks and line-of-fire profile); on `wasm32`,
  `map_draw_lanes` and `unit_symbology` (the overlay lane and marker atlas); over HTTP, the
  ballistics-catalog reads, the event list and the fire-mission routes of the operations domain.
- Used by: the route table in `apps/frontend/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the sidebar's "Field Tools" section in
  `crates/frontend/foundation/frontend_route_table/src/navigation_menu.rs`.
- Rules: the mortar page solves with the ballistics crates' solver, the same code the API re-solves a
  saved fire mission with, and never with a copy of it. No other crate imports a page's
  internals, so a page leaves with its route and its sidebar entry alone.

## Related documentation

- [Mortar calculator page](/documentation/crates/frontend/pages/field_tools_pages/mortar/mortar_calculator_page.md)
  — the mortar page's behaviour and design.
- [Operations domain](/crates/api/api_operations/src/README.md) — the fire-mission routes.
