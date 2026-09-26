# Mission approvals page

The `/admin/approvals` page, the [approvals](/documentation_v2/glossary/a_to_f.md#approvals) queue:
administrators work through the [missions](/documentation_v2/glossary/g_to_m.md#mission) waiting for
review, read the [artifact](/documentation_v2/glossary/a_to_f.md#artifact) each submission compiled into,
talk to its author in the review thread, and approve the artifact, approve it with conditions, or
reject it with a reason.

## Contents

```text
apps/website/frontend/src/v2/pages/administration/approvals/
├── decision_refusal.rs  a refused decision read into its reason and worded, and whether it is stale
├── mod.rs               the module tree; re-exports `MissionApprovalsPage`
├── page.rs              `MissionApprovalsPage`: the gate, the queue fetch and the shared desk
├── review_briefing.rs   the mission's briefing and its four settings tiles
├── review_decision.rs   the decision form and the body each decision sends
├── review_drawer.rs     `ReviewInspector`: header, briefing, artifact, review record, decision
├── submission_queue.rs  the queue heading and rows, and the split beside the drawer
└── tests/               unit tests for the queue's words, the decision bodies and the stale refusals
```

## How it works

`MissionApprovalsPage` renders `MissionApprovalsInner` inside `AdminGate`. The inner component
fetches the pending queue and builds the `ApprovalsDesk` the queue and the drawer share: which
submission is open (the first row until another is picked), the callback that reads the queue
again, and the notice a refused decision leaves. The notice lives on the desk, not in the drawer,
because reading the queue again rebuilds the drawer and the reviewer must still learn why the
decision did not land.

`ReviewInspector` is a component built per selected row, so its three fetches (the mission, the
review history and the artifact under review) belong to the row on screen and are disposed when
the row changes. It reuses the mission hub's review views, which the author reads the same record
through: the artifact provenance, the review list and thread, the comment composer and the review
wording. A decision is offered only while a review is under way and always names that review's
artifact; a conditional approval needs its conditions and a rejection its reason, trimmed and 1 to
8000 bytes. `decision_refusal.rs` reads `409 NO_PENDING_REVIEW`, `409 REVIEWED_ARTIFACT_CHANGED`
and a 409 without a code as a stale queue, which is read again; any other refusal shows the
[API](/documentation_v2/glossary/a_to_f.md#api)'s sentence. Every request runs in the browser build only;
a native build renders the failure branch.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/admin/approvals` | `MissionApprovalsPage` | route tier `admin`; the body renders inside `AdminGate`, for the `admin` [role](/documentation_v2/glossary/n_to_z.md#role) only | full-bleed inside the navigation frame; breadcrumb Administration / Mission Approvals; sidebar entry "Mission Approvals" |

## Data

- `GET /api/v1/approvals`: read as `Paginated<ApprovalRow>`; each row reads `mission_id`, `title`,
  `terrain`, `author_name`, `submitted_at` and, while a review is pending, `artifact_id`,
  `artifact_digest` and `version_semver`; the heading shows `total`. The page sends no `limit` or
  `offset`, so it reads the first page.
- `GET /api/v1/missions/{id}`: read as `MissionDetail`; the drawer reads `briefing`,
  `max_players`, `game_mode`, `weather` and `time_of_day`.
- `GET /api/v1/missions/{id}/artifacts/{artifact_id}`: read as `MissionArtifact`, the provenance
  and the compile diagnostics.
- `GET /api/v1/missions/{id}/reviews`: read as `MissionReviewHistory`, the reviews and the thread.
- `POST /api/v1/missions/{id}/review-comments`: sends a `ReviewCommentRequest` (`body`, and the
  `artifact_id` under review); the history is read again after it.
- `POST /api/v1/approvals/{id}/approve`: sends an `ApprovalDecision` (`artifact_id`, and
  `conditions` for a conditional approval).
- `POST /api/v1/approvals/{id}/reject`: sends a `RejectionDecision` (`artifact_id`, `reason`).
- The page reads the `AuthStore` context and the toast queue, and stores nothing in the browser.
  The workspace link opens `/missions/:id/artifacts/:artifact_id/workspace` in a new tab.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| below `admin` | "Admin access required." |
| queue loading | "Loading…" |
| queue failed | "Failed to load data." |
| empty queue | "Pending Review" with 0, "No pending approvals." and, in the drawer, "Queue clear — no pending approvals." |
| queue | "Pending Review" with the total; per row "[<date>]", the title, "By <author> · <terrain>", and "v<version> · artifact <digest> · submitted <UTC time>" or "Predates reviews — nothing to decide until its author resubmits it" |
| drawer header | the badges "Pending review", the terrain, the author, "v<version>", "artifact <digest>" and the local submission time, above the title |
| predates reviews | "This mission was submitted before reviews existed, so no artifact is under review and there is nothing to decide. Its author resubmits it from the mission hub, which compiles its current version into an artifact that appears here." |
| briefing | "Loading briefing…"; the briefing, or "The author submitted no briefing."; the tiles "Max Players", "Game Mode" ("COOP", "PvP" or "Zeus"), "Weather" and "Time of Day"; a failed read says "Could not load this mission's briefing — review it in the Mission Library before deciding." |
| artifact | "Artifact under review" and "Open the read-only review workspace"; "Loading the artifact…", the failure ("The artifact under review could not be read"), or the rows Version, Compiled, Compiler, Schema, Modpack, Terrain, Document size, Document SHA-256 and Artifact digest, then "The compile reported no findings." or "The compile reported N finding(s):" with each finding |
| review record | "Review record": "Loading the review record…", the failure ("The review record could not be read"), "No review has been opened yet." or each review; "Thread" with "Nobody has commented yet." or each comment; "Comment on the artifact under review — its author reads the thread." and "Post comment" |
| decision form | "Approve", "Approve with conditions" or "Reject"; the box "Conditions — what the approval holds the mission to" or "Reason — what the author must fix; it is all they are told"; "Send decision: <decision>"; "Write the conditions first", "Write the reason first", or "The reason is too long: N bytes, at most 8000" (likewise for the conditions) |
| decision landed | the toast "Approved — the mission is live and deployments run this artifact", "Approved with conditions — the mission is live and its author sees the conditions" or "Rejected — the mission is back with its author, with your reason" |
| queue stale | a notice atop the drawer: "Nothing of this mission is under review any more: it was decided, or its author is resubmitting it. The queue has been read again.", "The author resubmitted while you were reviewing, so your decision named an artifact that is no longer under review. Nothing was decided. Artifact <id> is under review now. The queue has been read again — review the new artifact before deciding.", or the API's sentence (else "The mission is not pending approval") followed by ". The queue has been read again." |
| decision refused | under the form, the API's sentence, else "The decision could not be sent" |

## Boundaries

- Depends on: `crate::v2::core::api` (`api_get`, `Paginated`, `ApprovalRow`, `MissionDetail`,
  `MissionArtifact`, `MissionReviewHistory`, `ApprovalDecision`, `RejectionDecision`, the
  `mission_reviews` endpoint module and `ApiRefusal`), `crate::v2::core::auth` (`AuthStore`),
  `crate::v2::core::ui` (`AdminGate`, `SplitPane`, `SplitPaneEmpty`, `MaterialIcon`, `cn`, the
  toast queue), `crate::v2::core::utils` (date and UTC formatting), and the review views in
  `apps/website/frontend/src/v2/pages/mission_hub/mission_review/`; over HTTP, the approval,
  mission and review routes of the [missions](/documentation_v2/glossary/g_to_m.md#missions) domain.
- Used by: the `/admin/approvals` route in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Mission Approvals" link in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; the DOM oracle's `approvals`
  capture in `tools_v2/developer-tools/src/browser_testing/dom_oracle/routes.rs`.
- Rules: every decision names the artifact under review
  (`every_decision_names_the_artifact_under_review`); a stale refusal reads the queue again and
  says why (`stale_decisions_reload_the_queue_and_say_why`,
  `the_decision_is_sent_with_the_artifact_and_stale_refusals_reload`); queue rows name what is
  under review (`queue_rows_name_what_is_under_review`), all in `tests/approvals.rs`.

## Related documentation

- [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
  — the page's behaviour, what each call means server-side, its design and decisions.
- [Missions domain](/apps/website/api_v2/src/missions/README.md) — the approval and review routes.
