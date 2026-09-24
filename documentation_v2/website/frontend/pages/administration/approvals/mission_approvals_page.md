**Status:** live

# Mission approvals page

The `/admin/approvals` page, titled Mission Approvals: administrators work through the
[missions](/documentation_v2/glossary.md#mission) waiting for review, read the
[artifact](/documentation_v2/glossary.md#artifact) each submission compiled into with its
provenance and compile findings, talk to the author in the review thread, and approve the artifact
into the live library, approve it with conditions, or reject it with a reason.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/administration/approvals/`](/apps/website/frontend/src/v2/pages/administration/approvals/):
  `page.rs` holds the route component `MissionApprovalsPage`, the queue fetch and the desk the
  queue and drawer share; `submission_queue.rs` the queue; `review_drawer.rs` the drawer, with
  `review_briefing.rs` for the briefing and settings; `review_decision.rs` the decision form;
  `decision_refusal.rs` the wording of a refused decision. The drawer reuses the mission hub's
  review views in
  [`apps/website/frontend/src/v2/pages/mission_hub/mission_review/`](/apps/website/frontend/src/v2/pages/mission_hub/mission_review/)
  (provenance, review history, comment box, review wording). The folder's
  [README](/apps/website/frontend/src/v2/pages/administration/approvals/README.md) describes each
  file.
- Entry: the `/admin/approvals` route renders `MissionApprovalsPage`
  (`apps/website/frontend/src/app_routes.rs`); `apps/website/frontend/src/router.rs` declares it
  for the `admin` tier, full-bleed, with the breadcrumb "Administration" › "Mission Approvals",
  and the sidebar lists it as "Mission Approvals"
  (`apps/website/frontend/src/v2/pages/navigation/nav_config.rs`).
- Related: the [approvals](/documentation_v2/glossary.md#approvals) glossary entry; the
  [mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md),
  where the author submits, reads the same review record and replies; the read-only review
  workspace ([README](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/README.md));
  the API's [missions domain](/apps/website/api_v2/src/missions/README.md); the
  [mission artifacts evidence](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md).

## Behaviour

1. The page body sits in `AdminGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`): "Loading
   session…" while the session restores, a sign-in prompt for a signed-out viewer, and "Admin
   access required." below the `admin` [role](/documentation_v2/glossary.md#role).
2. The queue loads on arrival: "Loading…" meanwhile, "Failed to load data." on failure. Its
   heading reads "Pending Review" beside the number of missions pending in total. Each row shows
   the submission date in brackets, the title, "By <author> · <terrain>" and either
   "v<version> · artifact <first 12 hex of the digest> · submitted <UTC time>" or, for a mission
   submitted before reviews existed, "Predates reviews — nothing to decide until its author
   resubmits it". An empty queue reads "No pending approvals." and the drawer "Queue clear — no
   pending approvals.".
3. The first row is open until the reviewer picks another. The drawer is built per row, so its
   fetches belong to the row on screen and a newly picked row never shows another row's answers.
4. The drawer's header carries the badges "Pending review", the terrain, the author, the version,
   the short artifact digest and the local submission time, above the mission title. A mission
   that predates reviews shows a notice instead of a decision: no artifact is under review, and
   its author resubmits it from the mission hub, which compiles its current version into an
   artifact that then appears here.
5. The briefing reads the mission itself: "Loading briefing…", then the author's briefing ("The
   author submitted no briefing." when empty) and four tiles, Max Players, Game Mode (COOP, PvP or
   Zeus), Weather and Time of Day. When the mission cannot be read the drawer says "Could not load
   this mission's briefing — review it in the Mission Library before deciding.".
6. "Artifact under review" lists the artifact's provenance, Version, Compiled, Compiler, Schema,
   Modpack, Terrain, Document size, Document SHA-256 and Artifact digest, then its compile
   findings ("The compile reported no findings." or "The compile reported N finding(s):" with
   each finding). The link "Open the read-only review workspace" opens the artifact on the map in
   a new tab, at `/missions/:id/artifacts/:artifact_id/workspace`.
7. "Review record" lists every earlier review of the mission and its "Thread". The comment box
   ("Comment on the artifact under review — its author reads the thread.") posts a comment tied to
   the artifact under review and reloads the record.
8. The decision form sits at the foot of the drawer while a review is under way. The reviewer
   picks "Approve", "Approve with conditions" or "Reject"; the last two open a text box, labelled
   "Conditions — what the approval holds the mission to" or "Reason — what the author must fix; it
   is all they are told", whose text is trimmed and must hold 1 to 8000 bytes ("Write the
   conditions first", "Write the reason first", or a too-long message). The button reads "Send
   decision: <decision>".
9. A decision that lands is toasted: "Approved — the mission is live and deployments run this
   artifact", "Approved with conditions — the mission is live and its author sees the conditions"
   or "Rejected — the mission is back with its author, with your reason", and the queue is read
   again. A refusal that means the queue is stale reads it again and leaves a notice at the top
   of the drawer: nothing is under review any more; the author resubmitted, so the decision named
   an artifact no longer under review (naming the one under review now); or the server's sentence
   that the mission is not pending approval. Any other refusal shows the server's sentence, else
   "The decision could not be sent".

## Data

The page README lists no calls, so the DTOs are named here. Server-side:

- `GET /api/v1/approvals` (`list_approvals` in
  `apps/website/api_v2/src/missions/handlers/approvals_queue.rs`): read as
  `Paginated<ApprovalRow>`; a row carries `mission_id`, `title`, `terrain`, `author_id`,
  `author_name`, `submitted_at` and, when a review is pending, `review_id`, `artifact_id`,
  `artifact_digest` and `version_semver`. The API lists the missions whose status is
  `pending_approval`, oldest submission first (the pending review's submission time, else the
  mission's update or creation time), 20 per page; the page reads only the first page, while
  `total` counts every pending mission.
- `GET /api/v1/missions/{id}` (`get_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_library.rs`): the `MissionDetail` behind the
  briefing and the tiles.
- `GET /api/v1/missions/{id}/artifacts/{artifact_id}` (`get_mission_artifact` in
  `apps/website/api_v2/src/missions/handlers/mission_reviews.rs`): the `MissionArtifact`, its
  provenance and its compile diagnostics.
- `GET /api/v1/missions/{id}/reviews` (`list_mission_reviews`): the `MissionReviewHistory`, every
  review with its decision, conditions or rejection reason, and the comment thread.
- `POST /api/v1/missions/{id}/review-comments` (`add_mission_review_comment`): a
  `ReviewCommentRequest`, recorded against the artifact under review.
- `POST /api/v1/approvals/{id}/approve` with an `ApprovalDecision` (`artifact_id`, optional
  `conditions`) and `POST /api/v1/approvals/{id}/reject` with a `RejectionDecision`
  (`artifact_id`, `reason`) (`approve_mission` and `reject_mission`, then `decide_review` in
  `apps/website/api_v2/src/missions/services/mission_reviews.rs`). In one transaction the API
  locks the mission, checks again that the caller is still an administrator (403 otherwise), and
  refuses with 409 when the mission is not pending approval, when no review is pending
  (`NO_PENDING_REVIEW`) or when the named artifact is not the one under review
  (`REVIEWED_ARTIFACT_CHANGED`, with the current `artifact_id` in its details). An approval sets
  the mission `live` with that artifact as its `approved_artifact_id` and records the review as
  `approved`, or as `approved_with_conditions` with the conditions in the thread; a rejection sets
  the mission `rejected` with the reason as its `rejection_reason` and in the thread. The API
  records `mission.approve` or `mission.reject`.

## Design

- A full-bleed split: the queue as the master column, the drawer as the detail, with the decision
  form pinned to the drawer's foot. The layout as built:

```text
+-------------------------------+---------------------------------------+
| PENDING REVIEW            2   | [Pending review] Everon Vance v0.4.0  |
| [Jul 24] Operation Cold Anvil | artifact bcfb3b1c4109  Fri Jul 24 …   |
|  By Vance · Everon            | OPERATION COLD ANVIL                  |
|  v0.4.0 · artifact bcfb3b1c…  |---------------------------------------|
| [Jul 24] Exercise Paper Tiger | Briefing … [Max][Mode][Weather][Time] |
|  By Rhodes · Arland           | ARTIFACT UNDER REVIEW  [Open review   |
|  Predates reviews — nothing   |                         workspace]    |
|  to decide until resubmitted  | Version · Compiler · Schema · Modpack |
|                               | Terrain · Size · SHA-256 · Digest     |
|                               | Findings: [rule][severity][subject]   |
|                               | REVIEW RECORD — reviews, thread       |
|                               | [ comment on the artifact… ]          |
|                               |                       [Post comment]  |
|                               |---------------------------------------|
|                               | (Approve)(Approve with conditions)    |
|                               | (Reject)  [ conditions / reason ]     |
|                               |               [Send decision: …]      |
+-------------------------------+---------------------------------------+
```

- Design target: the [mission approvals blueprint](/documentation_v2/website/frontend/pages/administration/approvals/visual_references/mission_approvals_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Mission Approvals section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#9-mission-approvals).
  The built page differs from the blueprint:
  - no heading or subtitle; the queue heading and the breadcrumb name the page;
  - queue rows show the version and artifact under review instead of author avatars;
  - the drawer reviews an immutable artifact: its provenance and compile findings replace the
    blueprint's map hero, and the settings tiles replace its BLUFOR, OPFOR and expected-duration
    counts;
  - "Open the read-only review workspace" replaces "Launch Tactical Planner for Deep Review";
  - three decisions with a required text replace "Request Changes" and "Approve & Publish".

## Open work

None.

## Decisions

- A decision names the artifact it was made on: an author may resubmit while a reviewer reads,
  and the API refuses a decision on an artifact that is no longer under review instead of
  approving something the reviewer never saw.
- The refused decision's notice lives on the desk the queue and drawer share, not in the drawer:
  reading the queue again rebuilds the drawer, and the reviewer must still learn why the decision
  did not land.
- A conditional approval needs its conditions and a rejection its reason: the text is what the
  author is told, so neither is sent empty.
