**Status:** live

# Mission library page

The `/missions` page, titled "Mission Library": signed-in members browse the community's
[missions](/documentation_v2/glossary.md#mission) in three scopes, open any mission's dossier in a
slide-over without leaving the list, and mission makers create a new mission there and hand it to
the [Mission Creator](/documentation_v2/glossary.md#mission-creator). A mission's author and
administrators also manage it from the dossier: upload a version, submit it for review, archive,
restore or delete it.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/mission_hub/library/`](/apps/website/frontend/src/v2/pages/mission_hub/library/):
  `page.rs` holds the route component `MissionLibraryPage`, the scope, search and filter state and
  both list fetches; `dossier_sheet.rs` and `dossier_body.rs` the slide-over dossier;
  `dossier_lifecycle.rs` the Manage row; `dossier_upload.rs` and `dossier_upload_panel.rs` the
  version upload. The New Mission dialog lives in
  [`apps/website/frontend/src/v2/pages/mission_hub/create_dialog/`](/apps/website/frontend/src/v2/pages/mission_hub/create_dialog/).
  Each folder's README describes its files.
- Entry: the `/missions` route, whose tier and layout the README's
  [Routes](/apps/website/frontend/src/v2/pages/mission_hub/library/README.md#routes) give.
- Related: the [mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md),
  whose read-only dossier body the slide-over renders; the review record shared with the
  [approvals](/documentation_v2/glossary.md#approvals) page, described in the
  [mission review record README](/apps/website/frontend/src/v2/pages/mission_hub/mission_review/README.md);
  the [review workspace page](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md)
  that the review record links; the [API](/documentation_v2/glossary.md#api)'s
  [missions domain](/apps/website/api_v2/src/missions/README.md).

## Behaviour

The page body sits in `AuthGate`, which shows the session states of the README's
[States](/apps/website/frontend/src/v2/pages/mission_hub/library/README.md#states) until a viewer
is signed in. Every text quoted below is listed there with the state that shows it.

### Browsing

1. The header holds the title, the line "Browse, filter, and deploy active operations across the
   theater.", the three scope tabs "Global Missions", "My Missions" and "Bookmarked", and, for a
   mission maker, the "New Mission" button.
2. The grid lists the missions of the selected scope. The tab, the search box and the three filter
   selects (terrain Everon or Arland; mode COOP, PvP or Zeus; players 1–8, 9–16, 17–32 or 33–64)
   key the fetch, so every keystroke and every change asks again. An unset filter is left out of
   the request.
3. The page asks for one page of the list and shows no paging control, so it shows at most the
   first 20 missions the API returns.
4. Above the grid the hero spotlights the first mission of the global scope with the same search
   and filters, whichever tab is open. Its "Live Operation" pulse shows only for a live mission;
   any other status shows as a muted chip, since the global scope includes the viewer's own
   drafts. An empty or whitespace briefing takes a fixed line naming the operation the priority
   deployment.
5. A card shows its artwork (a placeholder unless the thumbnail is an `http` or `https` URL), its
   mode and status badges, the author, the terrain and "<n> MAX". On the viewer's own returned
   mission it adds "Returned: <reason>" when the reviewer gave one.
6. The bookmark star on a card or in the dossier flips at once, then posts or deletes the bookmark
   and refetches the grid; a refused call flips it back and toasts.
7. With no missions, the grid says "No missions found.". A mission maker on "My Missions" with no
   search or filter sees "No missions yet" and a "New Mission" button instead.

### Creating a mission

1. "New Mission", the empty-state button or Ctrl or Cmd+N opens the New Mission dialog, for the
   `mission_maker` [role](/documentation_v2/glossary.md#role) and above only. The shortcut is
   ignored while an input, a text area or a select has focus, and opening the dialog closes the
   dossier first, so one overlay shows at a time.
2. The dialog asks for the "Operation Designation", the terrain (Everon or Arland, Everon first),
   the game mode ("Co-op PvE", "PvP", "Zeus"), the "Insertion Time" (14:00), the weather ("Clear
   (Default)", "Overcast", "Heavy Rain", "Dense Fog"), "Max Players" (16 to 128, 64 first) and an
   optional briefing. The author is the session, never a field, and there is no thumbnail field.
3. An empty title toasts "Title is required" and sends nothing. Otherwise "Create Mission Draft"
   (reading "Creating…" while it runs) creates the draft, toasts "Mission created", and loads
   `/missions/{id}/edit` as a full page, where the Mission Creator opens it.
4. Closing the dialog resets every field. A refusal toasts the API's sentence, or "Failed to create
   mission".

### The dossier

1. A card or the hero's "[ View Dossier ]" opens the dossier in a slide-over sheet. The mission is
   fetched at once, but the dossier renders only after the 320 ms slide, behind "Loading dossier…".
2. Two predicates decide what the viewer gets. `can_edit` is a mission maker who is the author, or
   an administrator: it shows "[ OPEN IN MISSION CREATOR ]", the upload panel and the
   collaboration buttons. `can_manage` is the author or an administrator at any role: it shows the
   returned-by-review notice, the review record and the Manage row.
3. The sections run in reading order: "Returned by review" with the reason (or "The reviewer did
   not leave a reason."), the overview's shared dossier body, the review record, "Version history",
   "Upload mission document", "Collaboration" and "Manage".
4. "Version history" shows the current version only, with its save time and a census of what it
   holds, and says that earlier versions are kept but cannot be listed.
5. "[ LAUNCH TACTICAL PLANNER ]" toasts "2D Tactical Planner — coming soon". "Comments" opens a
   sheet that says "Comments coming soon."; "Invite editor" opens a dialog with a disabled field
   and "Coming soon."; "Share for review" toasts "Will allow anyone to view and comment" and sends
   nothing.

### Uploading a version

1. The author picks a mission document. A file over 8 MiB is refused before a byte is read, with
   both sizes named and a pointer to saving from the Mission Creator instead.
2. The panel accepts an exported mission file or a bare editor payload, refuses a document that
   repeats a [slot](/documentation_v2/glossary.md#slot) id within a callsign, and suggests the
   next patch version after the current one.
3. It previews what the document changes against the current version, matching rows by `id`, so a
   reordered list is not an edit.
4. Upload posts the payload with the version number and the note "Uploaded from <file>"; the new
   version becomes the mission's current one, and a refusal lists the API's findings.

### Managing a mission

1. "Submit for review" (or "Resubmit for review" on a returned mission) shows on a draft or a
   returned mission. It compiles the current version into an
   [artifact](/documentation_v2/glossary.md#artifact) and opens its review; a refusal stays under
   the button with its reason and up to twenty findings.
2. "Archive mission" archives it; on an archived mission the button reads "Unarchive (restore to
   draft)".
3. "Delete mission" opens "Delete this mission?", which says that the mission and its versions
   leave the library for everyone and that deletion is refused while an
   [event](/documentation_v2/glossary.md#event) uses the mission. A delete closes the sheet.
4. Every write refetches the dossier and the grid.

### Known discrepancies

- The Manage row shows to an author at any role
  (`can_manage` in `apps/website/frontend/src/v2/pages/mission_hub/library/dossier_sheet.rs`,
  whose comment says the three routes test authorship "and nothing else"), but the API's submit,
  metadata `PATCH` and `DELETE` each require the `mission_maker` tier (`MissionMakerUser` and
  `lock_editable_mission` in `apps/website/api_v2/src/missions/services/mission_write_lock.rs`):
  an author demoted below mission maker sees the three buttons and gets 403 "insufficient role".
- "Bookmarked" lists every bookmarked mission that is not deleted, whatever its status
  (`push_filters` in `apps/website/api_v2/src/missions/handlers/mission_library.rs`), while the
  dossier fetch serves only a live mission or the viewer's own (`can_view` in
  `apps/website/api_v2/src/missions/validation/access.rs`): a bookmarked mission its author has
  since archived still shows its card, and its dossier reads "Failed to load data.".
- "Share for review" reports a success toast, "Will allow anyone to view and comment"
  (`apps/website/frontend/src/v2/pages/mission_hub/library/dossier_collaboration.rs`), but calls
  nothing: the API has no sharing route.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/mission_hub/library/README.md#data) lists
each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/missions` (`list_missions` in
  `apps/website/api_v2/src/missions/handlers/mission_library.rs`): any signed-in member. `global`
  holds the live missions plus the caller's own that are not archived; `mine` the caller's own at
  every status; `bookmarked` the caller's bookmarks. A filter value the API does not know is
  ignored rather than refused; `player_count` is an inclusive `lo-hi` range on `max_players`; `q`
  matches the title only, case-insensitively. Rows come newest-updated first, 20 by default and
  at most 100 with `limit`, from `offset`, with `total`; each card adds the author's name and
  avatar and whether the caller bookmarked it.
- `GET /api/v1/missions/{id}` (`get_mission`, same file): a live mission, or any mission for its
  author or an administrator; anyone else gets 404. It adds the armory rows and the current
  version with its full payload.
- `POST` and `DELETE /api/v1/missions/{id}/bookmark`: an idempotent insert and a delete of the
  caller's own bookmark row.
- `POST /api/v1/missions` (`create_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_lifecycle.rs`): `mission_maker` and above.
  It trims and requires the title, takes terrain `everon`, `arland` or `custom`, mode `pve_coop`,
  `pvp` or `zeus`, 1 to 256 players, weather defaulting to clear and a time defaulting to 14:00,
  then creates the draft and its first version, `0.1.0`, in one transaction and answers 201.
- `PATCH /api/v1/missions/{id}` (`update_mission`, same file): archives, or restores an archived
  mission to draft, and records `mission.archive` or `mission.unarchive`. Archiving is refused
  with 409 while an upcoming event uses the mission; restoring anything but an archived mission
  is refused with 409.
- `DELETE /api/v1/missions/{id}` (`delete_mission`, same file): a soft delete, refused with 409
  while any event uses the mission, recorded as `mission.delete_authorized`.
- `POST /api/v1/missions/{id}/versions` (`create_version` in
  `apps/website/api_v2/src/missions/handlers/mission_versions.rs`): a mission maker who is the
  author, or an administrator. The version must be valid SemVer and new to the mission (409
  "version already exists"), and the payload passes the schema and cargo checks and must not be
  empty. The version becomes current, the mission moves to the top of the library, and a title in
  the payload replaces the mission's title.
- `POST /api/v1/missions/{id}/submit` (`submit_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_submission.rs`): compiles the current version
  into an immutable artifact, opens its review and sets the mission to `pending_approval` in one
  transaction; a draft or a returned mission qualifies, a version that does not compile answers
  422, and a mission under review, live or archived answers 409.
- Every write above takes the mission's row lock and re-reads the caller's role inside it
  (`lock_editable_mission`), except the version upload, which checks role and authorship without
  the lock.

## Design

- The page draws on a topographic background with a glass tint. The hero spans the top; the
  toolbar under it holds the search box and the three selects; the grid of cards follows.
- The dossier is a sheet 60% of the viewport wide from the medium breakpoint (full width below)
  with a cinematic header (artwork, status, title, "Authored by <name>", bookmark and close) and a
  sticky footer of two buttons.
- Design target: the [mission library blueprint](/documentation_v2/website/frontend/pages/mission_hub/library/visual_references/mission_library_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Mission Library section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#5-mission-library).
  The built page differs from the blueprint:
  - a "New Mission" button joins the header, and the subtitle adds "across the theater";
  - the filters are three unlabelled selects ("All Terrains", "All Modes", "All Players") with no
    custom-map terrain, and the player ranges are 1–8, 9–16, 17–32 and 33–64 rather than 1-16,
    17-32 and 33-64;
  - the hero shows "<n> OPERATORS" with no terrain name, and "Live Operation" only for a live
    mission;
  - a card carries a status badge, a bookmark star and "<n> MAX", and opens the dossier in place;
  - no pagination row, which an earlier layout sketch drew under the grid.
- The archived spec's faction filter, "My Mission" pill and "[View Overview]" button are not built:
  the scope tabs replace the pill, and a card opens the dossier rather than the overview route.
- The comments, invite, share and planner controls are placeholders; no ticket is open for them.

## Open work

- [T-1030 — Fix mission version save, set-current and armory skipping write lock](/.ai/tickets/T-1030.toml)
  (idea, no plan): the version upload takes the mission's row lock like the other writes, so a
  concurrent delete, demotion or change of author cannot race it.
- [T-295 — Realtime collaborative editing](/documentation_v2/tickets/specs/t295_realtime_collab.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-295_plan.md)): a version post names the
  version it was edited from, and the API answers 409 when another save came first.
- [T-1053 — Decide whether mission upload and save share one duplicate-slot check](/.ai/tickets/T-1053.toml)
  (idea, no plan): the upload's duplicate slot id check and the Mission Creator's save agree.
- [T-1076 — Derive terrain checks and create dialog from the terrain registry](/.ai/tickets/T-1076.toml)
  (idea, no plan): the New Mission dialog offers only terrains that have map data; today it offers
  Arland, which has none.
- [T-1005 — Refactor frontend so core and pages stop importing apps/editor](/.ai/tickets/T-1005.toml)
  (idea, no plan): the upload stops importing the Mission Creator's code directly.
- [T-846 — role_notice query is written on editor denial but never read](/.ai/tickets/T-846.toml)
  (deferred, no plan): a viewer the route guard sends here from a `mission_maker` route learns why.

## Decisions

- The dossier opens in a sheet over the list: one click reads a mission, and closing it returns
  to the same scroll position and filters; `/missions/:id` keeps the standalone overview for deep
  links.
- Creation is a dialog over the library, not a route: a mission maker names the draft and its
  environment in one step and lands in the Mission Creator, and `/missions/create` does not exist;
  the [archived setup wizard page](/documentation_v2/archive/go_and_react_era_design/mission_creator_setup_wizard_page.md)
  records the page it replaced.
- The create controls read the signed-in, reactive role: a page that has not finished restoring
  its session never counts as a mission maker.
- The upload refuses a large file before reading it: the browser build is 32-bit and a parsed
  mission costs several times its source text, so a mission that large is saved from the Mission
  Creator instead.
- The upload preview compares rows by `id`, never by position, so reordering is not reported as a
  change.
