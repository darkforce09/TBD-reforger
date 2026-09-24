# Mission Approvals Queue (`/admin/approvals`)

The missions waiting on a reviewer, the immutable artifact each submission compiled into, and the
surface a decision is made on.

## Architecture
- **`page.rs`**: route component — fetches the pending queue (`GET /approvals`), puts it behind the
  administrator gate, and owns the desk the queue and drawer share: which submission is open, the
  refetch every decision calls, and the notice a refused decision leaves.
- **`submission_queue.rs`**: the queue heading with the backlog's total, and one selectable row per
  submission — with the version and short artifact digest under review and when that review was
  submitted, or, for a mission submitted before reviews existed, the note that nothing can be
  decided until its author resubmits it.
- **`review_drawer.rs`**: the drawer — the header, the stale-decision notice, the mission's briefing
  and settings (`review_briefing.rs`), the artifact under review with its provenance and compile
  findings (`GET /missions/:id/artifacts/:artifactId`) and the link to its read-only review
  workspace, and the review record (`GET /missions/:id/reviews`) with the thread and a comment box
  tied to the artifact under review (`POST /missions/:id/review-comments`).
- **`review_decision.rs`**: the decision form — approve, approve with conditions (conditions
  required), or reject (reason required) — each sending the artifact id of the pending review to
  `POST /approvals/:id/approve` or `/reject`.
- **`decision_refusal.rs`**: what a refused decision is told. `NO_PENDING_REVIEW`,
  `REVIEWED_ARTIFACT_CHANGED` (naming the artifact under review now) and "mission is not pending
  approval" read the queue again and leave their sentence on the desk.
- **`tests/approvals.rs`**: the queue's words against the captured queue, the decision bodies and
  refusals, and the drawer's wiring.

## Mission hub counterpart
The author reads the same review record — reviews, decisions, conditions, rejection reasons and the
thread — in the mission hub (`pages/mission_hub/mission_review/`), and replies there.

## Related documentation

- [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
  — the page's behaviour, what each call means server-side, its design, open work and decisions.
