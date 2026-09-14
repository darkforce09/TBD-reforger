# Vehicle Database (`/vehicles`)

Master-detail vehicle index: a faction-grouped list of vehicles beside the identification
dossier of the selected one.

## Architecture
- **`page.rs`**: route component — fetches `GET /vehicle-database`, owns the selection and the
  search text, and composes the split view.
- **`vehicle_grid.rs`**: the master list — the search box and one labelled group per faction,
  each row showing a vehicle's name and armour class.
- **`spec_drawer.rs`**: the dossier — the vehicle photograph or its placeholder icon, the armour,
  amphibious and faction chips, the primary-threat note, and the telemetry readouts.
- **`helpers.rs`**: the field read over the still-untyped vehicle payload.
- **`tests/vehicles.rs`**: the faction-grouping helper.

## Not present in the legacy page
- **`faction_filter.rs`**: there is no faction selector. Factions are section headings in the
  list, and every faction is always shown.
- **Card grid with 3D render thumbnails**: the master pane is a text list; the only image is the
  photograph on the dossier, and it comes from `profile_image_url`.
- **Slide-out spec drawer**: the dossier is the always-visible detail pane, not an overlay.
- **Armour values, seating, top speed, inventory capacity**: the wire carries `name`, `faction`,
  `armor_type`, `amphibious`, `primary_threat` and `profile_image_url` only, so the telemetry
  grid shows faction, armour class and amphibious.
