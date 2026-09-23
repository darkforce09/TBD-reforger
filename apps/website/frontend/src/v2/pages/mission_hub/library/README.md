# Mission Library Page (`/missions`)

## Architecture
- **`page.rs`**: Main layout file (~200 LOC).
- **`filter_bar.rs`**: Terrain dropdown (Everon, Arland, Kolgujev), game mode chips, player count range slider.
- **`search_bar.rs`**: Free-text scenario and author search.
- **`card_grid.rs`**: Responsive scenario cards displaying player count, terrain badge, and thumbnail.
- **`pagination.rs`**: Page navigation controls.

## Files
- **`page.rs`**: the route — the scope, search and filter state, both fetches, and the two overlays.
- **`header.rs`**: the title, the three scope tabs and the New Mission button.
- **`search_bar.rs`** / **`filter_bar.rs`**: the free-text search and the terrain / mode / player-count selects.
- **`featured_hero.rs`**: the cinematic hero over the grid.
- **`card_grid.rs`**: the body layout, the mission card, the status chip and the shared formatters.
- **`dossier_sheet.rs`** → **`dossier_body.rs`**: the slide-over dossier and its content, composed
  from **`dossier_lifecycle.rs`** (review verdict, manage row with the submission control that names
  each refusal reason and lists every finding, delete confirm), the review record
  (`mission_review/review_record.rs`) for the author and administrators,
  **`dossier_collaboration.rs`** (comments, invite), **`dossier_versions.rs`** (the version rail) and
  **`dossier_upload_panel.rs`** (upload a document as the next version).
- **`dossier_upload.rs`** and **`mission_diff.rs`**: the pure halves — the upload guards, and the
  structural comparison of two stored payloads.

**Not present in the legacy page:** pagination. The list endpoint is paged, but the page requests
one page and renders it, with no page controls — so there is no `pagination.rs`.
