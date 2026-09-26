**Status:** live

# Mission approvals page

The `/admin/approvals` page, titled Mission Approvals: administrators work through the
[missions](/documentation_v2/glossary/g_to_m.md#mission) waiting for review, read the
[artifact](/documentation_v2/glossary/a_to_f.md#artifact) each submission compiled into with its
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
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/administration/approvals/README.md#routes).
- Related: the [approvals](/documentation_v2/glossary/a_to_f.md#approvals) glossary entry; the
  [mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md),
  where the author submits, reads the same review record and replies; the read-only review
  workspace ([README](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/README.md));
  the [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [missions domain](/apps/website/api_v2/src/missions/README.md); the
  [mission artifacts evidence](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md).

## Behaviour

1. The page body sits in `AdminGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which
   shows the session and access states of the README's
   [States](/apps/website/frontend/src/v2/pages/administration/approvals/README.md#states) in
   place of the page until a signed-in viewer holds the `admin`
   [role](/documentation_v2/glossary/n_to_z.md#role).
2. The queue loads on arrival. Its heading counts the missions pending in total. Each row names
   the mission, its author and terrain, and either the version and short artifact digest under
   review with the UTC submission time or, for a mission submitted before reviews existed, that
   nothing can be decided until its author resubmits it. An empty queue says the queue is clear,
   in the list and in the drawer.
3. The first row is open until the reviewer picks another. The drawer is built per row, so its
   fetches belong to the row on screen and a newly picked row never shows another row's answers.
4. The drawer's header carries the review's badges above the mission title. A mission
   that predates reviews shows a notice instead of a decision: no artifact is under review, and
   its author resubmits it from the mission hub, which compiles its current version into an
   artifact that then appears here.
5. The briefing reads the mission itself: the author's briefing and four tiles, Max Players, Game
   Mode, Weather and Time of Day. When the mission cannot be read, the drawer sends the reviewer
   to the Mission Library before deciding.
6. "Artifact under review" lists the artifact's provenance, then its compile findings. The link
   to the read-only review workspace opens the artifact on the map in a new tab, at
   `/missions/:id/artifacts/:artifact_id/workspace`.
7. "Review record" lists every earlier review of the mission and its thread. The comment box
   posts a comment tied to the artifact under review and reloads the record.
8. The decision form sits at the foot of the drawer while a review is under way. The reviewer
   picks approve, approve with conditions or reject; the last two open a text box whose text is
   trimmed and must hold 1 to 8000 bytes.
9. A decision that lands is toasted, and the queue is read again. A refusal that means the queue
   is stale reads it again and leaves a notice at the top of the drawer: nothing is under review
   any more; the author resubmitted, so the decision named an artifact no longer under review
   (naming the one under review now); or the server's sentence that the mission is not pending
   approval. Any other refusal shows the server's sentence under the form. The README's
   [States](/apps/website/frontend/src/v2/pages/administration/approvals/README.md#states) quote
   every text.

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/administration/approvals/README.md#data)
lists each call with the DTO it reads or sends. Server-side:

- `GET /api/v1/approvals` (`list_approvals` in
  `apps/website/api_v2/src/missions/handlers/approvals_queue.rs`): the API lists the missions
  whose status is `pending_approval`, oldest submission first (the pending review's submission
  time, else the mission's update or creation time), 20 per page; the page reads only the first
  page, while `total` counts every pending mission.
- `GET /api/v1/missions/{id}` (`get_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_library.rs`): the mission behind the
  briefing and the tiles.
- `GET /api/v1/missions/{id}/artifacts/{artifact_id}` (`get_mission_artifact` in
  `apps/website/api_v2/src/missions/handlers/mission_reviews.rs`): the artifact's provenance and
  its compile diagnostics.
- `GET /api/v1/missions/{id}/reviews` (`list_mission_reviews`): every review with its decision,
  conditions or rejection reason, and the comment thread.
- `POST /api/v1/missions/{id}/review-comments` (`add_mission_review_comment`): records the
  comment against the artifact under review.
- `POST /api/v1/approvals/{id}/approve` and `POST /api/v1/approvals/{id}/reject`
  (`approve_mission` and `reject_mission`, then `decide_review` in
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
