# Vehicle database page

The `/vehicles` page: the vehicle identification index, a faction-grouped list beside the dossier
of the selected vehicle, with its photograph, armour class, amphibious flag and primary threat.

## Contents

```text
apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/
├── helpers.rs       `vstr`: the total string read over the untyped vehicle rows
├── mod.rs           the module tree; re-exports `VehicleDatabasePage`
├── page.rs          `VehicleDatabasePage`: the list fetch, selection, search and split view
├── spec_drawer.rs   the dossier: photograph or placeholder, chips, primary threat, telemetry
├── tests/           unit tests for the faction grouping
└── vehicle_grid.rs  the master pane: the search box and one labelled group per faction
```

## How it works

`VehicleDatabasePage` renders inside `AuthGate` and fetches the vehicle list once; both panes of
the `GlassSplit` read that one list. The selection starts on the first row and falls back to it
whenever the selected id names no vehicle. `faction_order` groups the rows by faction in the order
each faction first appears, which is the name order the [API](/documentation_v2/glossary/a_to_f.md#api)
returns; a row without a faction is left out of the list. The search matches a vehicle's name,
armour class or faction, and a group whose rows all filter out disappears. The dossier renders
every field as optional: an empty image URL shows a vehicle icon, an empty amphibious value drops
its chip and reads "—" in the readouts, and an empty threat shows a fixed line. The amphibious chip
is a warning for yes, `y` or `true`, a success for no, `n` or `false`, and neutral otherwise.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/vehicles` | `VehicleDatabasePage` | route tier `none`; the list renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Doctrine & Info / Vehicle Database |

## Data

- `GET /api/v1/vehicle-database`: read as `DataEnvelope<serde_json::Value>`; each row reads `id`,
  `name`, `faction`, `armor_type`, `amphibious`, `primary_threat` and `profile_image_url`.
- The page reads the `AuthStore` from context and writes nothing. The fetch runs in the browser
  build only; a native build shows the failure line.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load vehicles." |
| empty | "No vehicles in the database." |
| loaded | "Vehicle Database" and the "Search assets..." box over the faction groups, each row showing a name and armour class; the dossier with "ARMOR: <class>", "AMPHIB: <value>" when set, the faction chip, the name, "Primary Threat" and "Telemetry" with "Faction", "Armor" and "Amphibious" |
| no threat recorded | "No primary threat recorded." |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `DataEnvelope`) and `crate::v2::core::ui`
  (`AuthGate`, the `split_pane` primitives, `MaterialIcon`), and the `AuthStore` context.
- Used by: the `/vehicles` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Vehicle Database" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; the DOM oracle's `vehicles`
  capture in `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: groups keep the order each faction first appears (`factions_preserve_first_seen_order` in
  `tests/vehicles.rs`); a missing field reads as an empty string through `vstr`, never as an error;
  both panes read the one fetched list.

## Related documentation

- [Vehicle database page](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md)
  — the page's behaviour and design.
- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the vehicle
  database routes.
