# Mission Library Page (`/missions`)

*Replaces the 3,663-line monolithic `editor/library/mission_library.rs` file.*

## Architecture
- **`page.rs`**: Main layout file (~200 LOC).
- **`filter_bar.rs`**: Terrain dropdown (Everon, Arland, Kolgujev), game mode chips, player count range slider.
- **`search_bar.rs`**: Free-text scenario and author search.
- **`card_grid.rs`**: Responsive scenario cards displaying player count, terrain badge, and thumbnail.
- **`pagination.rs`**: Page navigation controls.
