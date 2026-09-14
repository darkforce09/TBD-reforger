# Mortar Ballistics Calculator (`/tools/mortar`)

Coordinate entry, a tube, and the firing solution the server computes for them — optionally saved
against a scheduled operation so it survives a reload.

## Architecture
- **`page.rs`**: the route component. Owns every shared signal, the operation list and saved
  fire-mission fetches, the effects that reconcile the remembered operation and hydrate the form
  from the newest saved row, the Calculate request, and the arrangement of the panels.
- **`weapon_selector.rs`**: the two dropdowns at the head of the inputs card — the weapon system,
  and the operation a solution is saved to.
- **`map_picker.rs`**: the shared form-control class, the numeric coordinate fields, and the
  preview box — grid backdrop, gun-target line, and the two markers.
- **`firing_solution.rs`**: the result card — distance, azimuth, elevation, propellant charge and
  time of flight, plus whether those numbers survive a reload.
- **`saved_fires.rs`**: the stored fire-mission types, the request body, the rules for restoring a
  row into the form, the remembered-operation preference, and the saved-fire-missions panel.
- **`grid.rs`**: the pure maths — the grid encoding and its inverse, the preview projection, and
  the thousands-separated integer the cards print. This file moves under the map engine when the
  engine folder lands.
- **`tests/mortar.rs`**: the save round trip over captured server bytes, the grid encoding's
  losslessness, the preview perturbation guard, and the hydration latch.

## Not present in the legacy page
- **A propellant charge selector**: the charge is computed by the server and *displayed* by the
  solution card. There is nothing to pick.
- **A dispersion table**: the solution card shows distance, azimuth, elevation, charge and time of
  flight, and no dispersion figures.
- **MGRS coordinate entry**: the four inputs are plain metric x/y numbers. The grid *string* stored
  on the row is `"x, y"`, deliberately lossless, and is not a military grid reference.
- **Terrain selection**: the preview is a fitted abstract frame with no terrain behind it — the
  page has no mission and no map.
