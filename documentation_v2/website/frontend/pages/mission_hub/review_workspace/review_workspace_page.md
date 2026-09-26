**Status:** live

# Review workspace page

The `/missions/:id/artifacts/:artifact_id/workspace` page, the review workspace of an
[artifact](/documentation_v2/glossary/a_to_f.md#artifact): the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) opened read-only on exactly the
version the artifact compiled from, so a reviewer or the
[mission](/documentation_v2/glossary/g_to_m.md#mission)'s author inspects what was submitted with every
editor tool while nothing is saved.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/mission_hub/review_workspace/`](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/):
  `page.rs` holds the route component `ReviewWorkspacePage`, the workspace read and the mounted
  editor; `banner.rs` the banner over it. The editor's read-only state lives in
  `apps/website/frontend/src/v2/apps/editor/shell/review_mode.rs`. The folder's
  [README](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/README.md) describes
  each file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/README.md#routes).
  Links to it come from the review record's history, on the
  [mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md)
  and in the library's dossier, and from the review drawer of the
  [mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md).
- Related: the [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md),
  whose editor this page mounts.

## Behaviour

### Opening the workspace

1. The route requires the `mission_maker` [role](/documentation_v2/glossary/n_to_z.md#role); a viewer
   below it is sent to the mission's overview at `/missions/:id?role_notice=mission_maker`. The
   page body then sits in `AuthGate`, whose session states are in the README's
   [States](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/README.md#states).
2. The page reads `:id` and `:artifact_id` once and fetches the workspace, showing "Opening the
   review workspace…" meanwhile. It fetches once per visit.
3. Only once the workspace has arrived does the page open the editor's review mode on the reviewed
   version and mount the Mission Creator, so the editor's boot finds that version instead of
   restoring a local draft.
4. A refused read shows a sentence for the status and a "Back to the mission" link to the
   mission's overview: 401 says the session has ended, 403 that only the author and
   administrators may open the workspace, 404 that the artifact does not exist or the mission is
   hidden, and any other failure shows the [API](/documentation_v2/glossary/a_to_f.md#api)'s message or
   "The review workspace could not be opened".

### Inspecting the version

1. The banner stays over the editor for the whole visit, because the editor otherwise looks like
   the Mission Creator: "Review workspace of artifact <short digest>, version <semver>", the
   read-only notice, the mission title with the compile time and the document digest, and a
   disclosure listing the compile findings (rule, severity, subject, message).
2. The document carries the mission-row fields the artifact's compile read, not the mission's
   current ones.
3. Every editor tool works, but review mode withholds every write: no draft record, no warm-session
   marker, no writer election, no version, no mission-row change and no unsaved-work prompt on
   leaving. Edits live in the tab's memory and are discarded when it closes.
4. Leaving the route closes review mode, so the next editor mount on `/missions/:id/edit` behaves
   normally.

### Known discrepancies

- The route requires the `mission_maker` role (the `/missions/:id/artifacts/:artifact_id/workspace`
  row in `apps/website/frontend/src/router.rs`), but the API serves the workspace to the mission's
  author at any role (`get_review_workspace` in
  `apps/website/api_v2/src/missions/handlers/mission_reviews.rs` takes `AuthUser` and checks
  authorship): an author demoted below mission maker is sent back to the overview, although the
  API would answer.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/README.md#data)
lists the call and its DTO. Server-side:

- `GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace` (`get_review_workspace` in
  `apps/website/api_v2/src/missions/handlers/mission_reviews.rs`): any signed-in viewer who may
  see the mission and who is its author or an administrator; a hidden mission answers 404 and a
  visible one the caller does not own 403. It loads the artifact of that mission (404 for another
  mission's artifact), then the version the artifact names, recomputes the SHA-256 of the stored
  payload and compares it with the digest the artifact recorded at compile time; a mismatch
  answers 500 "the reviewed version no longer matches its artifact", which the page shows as the
  API's message. The reply is the artifact with its metadata and compile diagnostics, and the
  version with its semver and payload.

## Design

- Full-bleed and chromeless: the Mission Creator fills the window with its own command strip and
  docks, and the banner is fixed and centred under the command strip, above the editor's chrome
  and below its dialogs.
- The design is the Mission Creator's own; the page adds only the banner and the failure screen.
  No visual reference set exists for it.

## Open work

- [T-1005 — Refactor frontend so core and pages stop importing apps/editor](/.ai/tickets/T-1005.toml)
  (idea, no plan): the page stops importing `MissionEditorPage` and `review_mode` from the Mission
  Creator directly, through a shared module.

## Decisions

- The workspace is the Mission Creator itself in a read-only mode, not a copy: the reviewer uses
  the same tools the author used, and a single cell that every write path consults keeps review
  mode from saving anything.
- The editor mounts only after the workspace read, so its boot always opens the reviewed version
  and never a draft the tab holds for the same mission.
- The API checks the stored version against the artifact's recorded digest on every read: a
  reviewer inspects exactly the bytes the artifact compiled, or gets an error.
