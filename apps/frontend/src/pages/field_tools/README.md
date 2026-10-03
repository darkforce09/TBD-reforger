# Field tools pages

The standalone tactical aids: pages that stand on their own rather than hanging off a
[mission](/documentation/glossary/g_to_m.md#mission) or an [event](/documentation/glossary/a_to_f.md#event).
The folder holds one page, the mortar calculator.

## Contents

```text
apps/frontend/src/pages/field_tools/
├── mod.rs   the module tree
└── mortar/  the `/tools/mortar` page: catalog weapon, guns and target in, a local firing solution out
```

## How it works

Each page here owns a route under `/tools/`, its own fetches and its own signals; outside the
folder only the route table and the sidebar's "Field Tools" section refer to it, so a page can be
removed without touching the rest of the tree. The mortar calculator reads the public
ballistics catalogs of the [operations](/documentation/glossary/n_to_z.md#operations) domain of the
[API](/documentation/glossary/a_to_f.md#api) (or the offline copy of them), solves the firing
solution on the device with the ballistics crates' solver, and lists the fire missions saved against an
event for a signed-in viewer. The debug benches at `/debug/building-viewer`, `/debug/world-los`
and `/debug/ballistics-agreement` are apps, in `apps/frontend/src/workspaces/debug/`, not field tools.

## Public surface

- `mortar::MortarCalculatorPage`: the route component `apps/frontend/src/app_routes.rs`
  binds to `/tools/mortar`.

## Boundaries

- Depends on: `crate::foundation::transport`, `crate::foundation::auth` (the `AuthStore` context),
  `crate::foundation::map_view`, `crate::foundation::offline`, `crate::foundation::ui` and
  `crate::foundation::utils`; `fire_mission_planning`, `ballistics_solver` and `ballistics_model`
  (the ballistics solver); `map_coordinates` (grid references); `map_engine` (the map views);
  over HTTP, the ballistics-catalog reads, the event list and the fire-mission routes of the
  operations domain.
- Used by: the route table in `apps/frontend/src/app_routes.rs` and
  `apps/frontend/src/foundation/route_table/mod.rs`; the sidebar's "Field Tools" section in
  `apps/frontend/src/foundation/route_table/navigation_menu.rs`.
- Rules: the mortar page solves with the ballistics crates' solver, the same code the API re-solves a
  saved fire mission with, and never with a copy of it. No other folder imports a page's
  internals, so a page leaves with its route and its sidebar entry alone.

## Related documentation

- [Mortar calculator page](/documentation/apps/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the mortar page's behaviour and design.
- [Operations domain](/crates/api/api_operations/src/README.md) — the fire-mission routes.
