# Mortar Ballistics Calculator (`/tools/mortar`)

*Replaces the 1,425-line monolithic `pages/public/mortar.rs` file.*

## Architecture
- **`page.rs`**: Main layout file.
- **`map_picker.rs`**: Terrain selection and coordinate input (MGRS or metric X/Y).
- **`weapon_selector.rs`**: Weapon system (M252 81mm, 2B14 82mm) and propellant charge selection.
- **`firing_solution.rs`**: Computed elevation (mils), azimuth (mils), time of flight, and dispersion table.
