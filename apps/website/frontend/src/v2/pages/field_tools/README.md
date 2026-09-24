# Field tools pages

The standalone tactical aids: pages that stand on their own rather than hanging off a
[mission](/documentation_v2/glossary.md#mission) or an [event](/documentation_v2/glossary.md#event).
The folder holds one page, the mortar calculator.

## Contents

```text
apps/website/frontend/src/v2/pages/field_tools/
├── mod.rs   the module tree; declares the `mortar` page module
└── mortar/  the `/tools/mortar` page: coordinates in, the server's firing solution out
```

## How it works

Each page here owns a route under `/tools/`, its own fetches and its own signals; outside the
folder only the route table and the sidebar's "Field Tools" section refer to it, so a page can be
removed without touching the rest of the tree. The mortar calculator sends its geometry to the
fire-mission routes of the [operations](/documentation_v2/glossary.md#operations) domain of the
[API](/documentation_v2/glossary.md#api), which solve the ballistics, and saves the answer against
an event when one is selected. The debug benches at `/debug/building-viewer` and `/debug/world-los`
are apps, in `apps/website/frontend/src/v2/apps/debug/`, not field tools.

## Public surface

- `mortar::MortarCalculatorPage`: the route component `apps/website/frontend/src/app_routes.rs`
  binds to `/tools/mortar`.

## Boundaries

- Depends on: `crate::v2::core::api`, `crate::v2::core::auth` (the `AuthStore` context),
  `crate::v2::core::ui` and `crate::v2::core::utils`; over HTTP, the event list and the
  `/api/v1/fire-missions` routes of the operations domain.
- Used by: the route table in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Field Tools" section in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`.
- Rules: no page here computes ballistics; the server solves them, and the mortar page sends the
  geometry and renders the reply. No other folder imports a page's internals, so a page leaves with
  its route and its sidebar entry alone.

## Related documentation

- [Mortar calculator page](/documentation_v2/website/frontend/pages/field_tools/mortar/mortar_calculator_page.md)
  — the mortar page's behaviour and design.
- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the fire-mission routes.
