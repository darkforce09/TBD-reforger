# Mission library page

The `/missions` page: the catalogue of [missions](/documentation_v2/glossary.md#mission) in three
scopes with search and filters, a featured mission above the grid, and a slide-over dossier from
which an author uploads versions, submits for review, archives or deletes a mission and opens it in
the [Mission Creator](/documentation_v2/glossary.md#mission-creator).

## Contents

```text
apps/website/frontend/src/v2/pages/mission_hub/library/
├── card_grid.rs              the body layout, the mission card and its bookmark, shared formatters
├── dossier_body.rs           the loaded dossier: header, sections in reading order, footer actions
├── dossier_collaboration.rs  the collaboration buttons, comments panel and invite dialog
├── dossier_lifecycle.rs      the returned-by-review notice, the Manage row, the delete dialog
├── dossier_sheet.rs          `MissionDossierSheet`: the mission fetch and the edit and manage gates
├── dossier_upload.rs         the upload's pure half: size budget, parse, next version, wording
├── dossier_upload_panel.rs   the panel that uploads a mission document as the next version
├── dossier_versions.rs       the version rail and the census of what a version holds
├── featured_hero.rs          the hero: the newest global mission and its dossier button
├── filter_bar.rs             the terrain, game-mode and player-count filters
├── header.rs                 the title, the three scope tabs (`SCOPES`) and the New Mission button
├── mission_diff.rs           the structural comparison of two stored mission payloads
├── mod.rs                    the module tree; re-exports `MissionLibraryPage`
├── page.rs                   the route component: scope, search and filter state, fetches, overlays
├── search_bar.rs             the free-text search box
└── tests/                    unit tests for the hero, card, bookmark, diff and upload
```

## How it works

`MissionLibraryPage` renders inside `AuthGate`. The scope tab, the query and the three filters key
the grid's fetch, so every keystroke and every select fetches again; a second fetch over the
`global` scope with the same filters keeps the hero on the newest global mission, whichever tab is
open. An empty filter is left out of the query. The page requests one page of the list and has no
paging controls. The New Mission button and the Ctrl or Cmd+N shortcut (ignored while a field has
focus) open the create dialog from `apps/website/frontend/src/v2/pages/mission_hub/create_dialog/`,
for the `mission_maker` [role](/documentation_v2/glossary.md#role) and above, read through the
authenticated reactive role so a page still bootstrapping never counts as a maker. Only one overlay
is open at a time: opening the dialog closes the dossier.

A card or the hero opens `MissionDossierSheet` in a `Sheet` without leaving the route. It fetches
the mission and withholds the heavy dossier until the slide finishes. `can_edit` (a mission maker
who is the author, or an administrator) gates the Mission Creator link, the collaboration controls
and the upload panel; `can_manage` (the author or an administrator, any role) gates the Manage row,
the review feedback and the review record, as the [API](/documentation_v2/glossary.md#api)'s own
predicate does. The dossier renders the overview page's read-only `dossier_body`, the review record
and submit control from `apps/website/frontend/src/v2/pages/mission_hub/mission_review/`, the
version rail, the upload panel, the collaboration section and the Manage row, in that order.

The upload refuses a file over 8 MiB (`UPLOAD_MAX_BYTES`) before reading a byte, accepts an
exported mission file or a bare editor payload, suggests the next patch version, previews what the
document changes with `mission_diff`, and posts the body `version_body_to_writer` from the map
engine writes. `mission_diff` matches rows by `id`, never by position, so a reordered array is not
an edit, and its samples stop at `DIFF_SAMPLE_CAP`. The version rail shows only the current version,
since the API has no route that lists versions. Comments and invites have no endpoint, and their
surfaces say so. The payload, which can reach hundreds of megabytes, is read by reference and never
cloned.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions` | `MissionLibraryPage` | route tier `none`; the data renders only for a signed-in viewer; creating a mission for `mission_maker` and above; the dossier's actions for the author or an administrator | full-bleed inside the navigation frame; breadcrumb Mission Hub / Mission Library |

## Data

- `GET /api/v1/missions?scope=<global|mine|bookmarked>`, with `terrain`, `mode`, `player_count` and
  `q` when set: the grid, read as `Paginated<MissionCard>`; the same call with `scope=global` feeds
  the hero.
- `GET /api/v1/missions/{id}`: the dossier, read as `MissionDetail`.
- `POST /api/v1/missions/{id}/bookmark` and `DELETE` on the same path: the bookmark on a card and in
  the dossier.
- `PATCH /api/v1/missions/{id}` with `{ "status": "archived" }`, or `"draft"` to restore it.
- `DELETE /api/v1/missions/{id}`: delete, after the confirmation dialog.
- `POST /api/v1/missions/{id}/versions`: the uploaded document as the next version (semver, the
  note "Uploaded from <file>" and the payload), sent and answered raw, so the payload echoed back is
  never parsed.
- `POST /api/v1/missions/{id}/submit` through `SubmitForReview`, the review record's calls through
  `MissionReviewRecord`, and `POST /api/v1/missions` through the create dialog.
- The page reads the session (the viewer's Discord id and role) from the `AuthStore` context. It
  does not read the `role_notice` query parameter the route guard adds when it sends a viewer below
  `mission_maker` here. Every call runs in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading…" |
| failed | "Failed to load data." |
| loaded | "Mission Library", "Browse, filter, and deploy active operations across the theater.", the tabs "Global Missions", "My Missions" and "Bookmarked", "New Mission" for a maker, the hero ("Live Operation" only for a live mission, "N OPERATORS", "[ View Dossier ]"), "Search operations...", "All Terrains", "All Modes", "All Players" and the cards |
| no missions | "No missions found."; for a maker on "My Missions" with no filter, "No missions yet", "Create a draft to open the Mission Creator." and "New Mission" |
| returned mission | "Returned: <reason>" on the viewer's own card, when a reason was given |
| bookmark | a toast "Mission bookmarked" or "Bookmark removed"; "Could not bookmark mission" or "Could not remove bookmark" on failure |
| dossier loading | "Loading dossier…" |
| dossier failed | "Failed to load data." |
| dossier loaded | "Authored by <name>", the shared body, "Returned by review" with the reason or "The reviewer did not leave a reason.", "Version history", "Upload mission document", "Collaboration", "Manage", and the footer "[ OPEN IN MISSION CREATOR ]" and "[ LAUNCH TACTICAL PLANNER ]" |
| planner | the toast "2D Tactical Planner — coming soon" |
| manage | "Submit for review" (or "Resubmit for review"), "Archive mission" (or "Unarchive (restore to draft)"), "Delete mission"; toasts "Mission archived", "Mission restored to draft", "Mission deleted", or "Could not archive mission", "Could not unarchive mission", "Could not delete mission" |
| delete confirmation | "Delete this mission?" with "Cancel" and "Delete mission" |
| upload | "Reading <file>…", "<file> — <census>. Ready to upload.", "Uploading v<semver>…", "Uploaded v<semver> — it is now this mission's current version."; refusals such as "Choose a mission document first.", "A version number is required (e.g. 1.2.3).", "Rejected (<status>): …" with the findings listed |

## Boundaries

- Depends on: `crate::v2::core::api` (the client, `MissionCard`, `MissionDetail`, `Paginated`),
  `crate::v2::core::auth` (`AuthStore`, `has_min_role_authed`, `Role`, `url_guard`),
  `crate::v2::core::ui` (`AuthGate`, `Sheet`, `Dialog`, `MaterialIcon`, `badge_class`, the toasts),
  the overview page's `dossier_body` and formatters, the review record's `MissionReviewRecord` and
  `SubmitForReview`, the create dialog's `CreateMissionDialog`,
  `website_map_engine::data::scenario::compile::version_body_to_writer`, and
  `shell::mission_size::format_bytes` from the Mission Creator in
  `apps/website/frontend/src/v2/apps/editor/`: this page imports the editor app directly.
- Used by: the `/missions` route in `apps/website/frontend/src/app_routes.rs`;
  `mission_library_source` in `apps/website/frontend/src/v2/core/test_support/pins.rs`, which the
  overview page's tests also read.
- Rules: the create affordances read the authenticated reactive role
  (`maker_affordance_uses_authed_reactive_role` in `tests/mission_library.rs`); a thumbnail or
  avatar `src` is only ever an `http(s)` URL (`mission_art_falls_back_for_non_http_thumbnails`,
  `author_avatar_emits_src_only_for_http_urls`); reordering rows is not a change
  (`reordering_rows_is_not_a_change` in `tests/mission_library_versions.rs`); the size gate runs
  before the file is read (`the_size_gate_names_both_numbers_and_is_inclusive_at_the_budget`).

## Related documentation

- [Mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md)
  — the page's behaviour and design.
