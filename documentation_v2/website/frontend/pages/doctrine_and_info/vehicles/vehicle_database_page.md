**Status:** live

# Vehicle database page

The `/vehicles` page, titled "Vehicle Database": signed-in members look up the vehicles the
community fields and faces, grouped by faction, and read one vehicle's identification dossier: its
armour class, whether it swims and what threatens it. The page only reads.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/`](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/):
  `page.rs` holds the route component `VehicleDatabasePage` and the list fetch; `vehicle_grid.rs`
  the faction-grouped list; `spec_drawer.rs` the dossier. The folder's
  [README](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/README.md) describes each
  file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/README.md#routes).
- Related: the [doctrine wiki page](/documentation_v2/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md),
  which holds the written manuals; the [API](/documentation_v2/glossary.md#api)'s
  [community content domain](/apps/website/api_v2/src/community_content/README.md), which owns the
  vehicle rows.

## Behaviour

The page body sits in `AuthGate`; the session, loading, failure and empty texts are in the
README's [States](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/README.md#states).

1. The page fetches the whole vehicle list once, and both panes read it.
2. The list, headed "Vehicle Database", groups the vehicles by faction in the order each faction
   first appears; since the API sorts by name, a faction's place follows its first vehicle by
   name. A row without a faction is left out. Each row shows the name and the armour class.
3. "Search assets..." narrows the list to vehicles whose name, armour class or faction matches; a
   faction whose rows all drop out disappears.
4. The first vehicle is selected on arrival, and the selection falls back to it whenever the
   selected vehicle is no longer in the list.
5. The dossier shows the profile image, or a vehicle icon when the vehicle has no image URL, then
   "ARMOR: <class>", "AMPHIB: <value>" when one is recorded, the faction chip and the name,
   "Primary Threat" (or "No primary threat recorded."), and "Telemetry" with "Faction", "Armor" and
   "Amphibious", where a missing amphibious value reads "—".
6. The amphibious chip warns for yes, `y` or `true`, reads as success for no, `n` or `false`, and
   stays neutral for anything else.

### Known discrepancies

- A profile image URL that fails to load shows the browser's broken image: the dossier falls back
  to the vehicle icon only when the URL is empty (`spec_drawer.rs` in
  `apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/` sets no error handler), and the
  API stores any text as `profile_image_url` without checking it is an `http` or `https` URL
  (`create_vehicle` in `apps/website/api_v2/src/community_content/handlers/vehicle_database.rs`),
  unlike the [mission](/documentation_v2/glossary.md#mission) library, which guards its
  thumbnails on both sides.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/README.md#data)
lists the call and the fields the page reads. Server-side:

- `GET /api/v1/vehicle-database` (`list_vehicles` in
  `apps/website/api_v2/src/community_content/handlers/vehicle_database.rs`): any signed-in member;
  every vehicle row ordered by name, with an empty string for a missing amphibious value, threat or
  image.
- `POST /api/v1/vehicle-database` (`create_vehicle`, same file): `admin` only; requires a
  non-empty name, faction and armour class, trims every field and stores an empty optional field
  as null. No page calls it: vehicle rows are added through the API or the seeds, and the API has
  no route to change or remove one.

## Design

- A `GlassSplit` with an 18rem master list and the dossier as its detail. The dossier opens with a
  wide image area, then the chips, the name, the threat panel and the telemetry grid.
- The dossier shows only the stored fields; it invents no speed, crew, armament or tactical
  directive.
- Design target: the [vehicle dossier blueprint](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/visual_references/vehicle_dossier_blueprint/README.md)
  and the [BTR-70 render](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/visual_references/btr_70_render/README.md),
  both design-phase references. The built page differs from the blueprint:
  - no wiki-style sidebar of manual sections, no "Publish Revision", "Archived" or "Export PDF"
    actions;
  - the list is one searchable column grouped by the stored faction, with no flags;
  - the dossier has no description paragraph, no "CRITICAL" identification warning, no mobility,
    defence or capacity figures, no armament list and no row of threat icons: the stored row holds
    none of them;
  - the image area shows the stored profile image, not a render.

## Open work

- [T-940.8 — Vehicles: PUT, PATCH, DELETE routes and DTOs](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_8_plan.md)): vehicle rows become editable
  and removable through the API, and a deleted vehicle leaves the list.

## Decisions

- The page is read-only and shows only what a row stores: a missing value reads as absent, never
  as an invented figure, so the dossier cannot mislead a player identifying a vehicle.
- Vehicle identification is its own page, not a wiki manual: its rows are structured data with a
  faction and an armour class to search and group by.
- One fetch feeds both panes: search and selection cost no request.
