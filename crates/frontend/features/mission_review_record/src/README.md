# Mission review record

One rendering of a [mission](/documentation/glossary/g_to_m.md#mission)'s review, shared by every page
that shows it: the review history and thread with a reply box, an
[artifact](/documentation/glossary/a_to_f.md#artifact)'s provenance and compile findings, the submit
control with its refusal panel, and the wording under all of them, so the author and the reviewer
read the same record in the same words.

## Contents

```text
crates/frontend/features/mission_review_record/src/
├── artifact_provenance_view.rs  an artifact's provenance rows and the list of its compile findings
├── comment_composer.rs          `ReviewCommentComposer`: the reply box that posts a thread comment
├── error.rs                     `Error` and `Result`: why a body of review text cannot be sent
├── lib.rs                       the crate root: the module tree and the root re-exports
├── prelude.rs                   the components, views and wording most pages name
├── review_history_view.rs       every review, newest first, then the thread in writing order
├── review_record.rs             `MissionReviewRecord`: history read, approved artifact and reply box
├── review_wording.rs            the pure wording: states and tones, digests, sizes, findings, links
├── submission_action.rs         `SubmitForReview`: the submit button and the refusal panel under it
├── submission_refusal.rs        a refused submission read into its reason and its listed findings
└── tests/                       unit tests for the review wording and the submission refusals
```

## How it works

`MissionReviewRecord` shows under the mission overview's dossier and in the library's dossier
sheet, for the author and administrators, the two the [API](/documentation/glossary/a_to_f.md#api)
serves the history to. It reads the history unasked only once the mission row shows a review
happened (otherwise "This mission has not been submitted for review." and "Look for earlier
reviews"), and again after a posted reply. It names the approved artifact ("… — deployments run
exactly these bytes."), then the history, the thread and the reply box, whose comment concerns the
newest review's artifact. A mission awaiting approval with no review under way predates reviews,
and the record offers "Resubmit for review".

`SubmitForReview` compiles the current version into an artifact and opens its review. Its refusal
stays until the next attempt: `submission_refusal` reads a 422 whose `details.code` is
`NO_PLACED_SLOTS`, `UNCOMPILABLE_VERSION`, `DOCUMENT_CONTRACT_VIOLATION` or
`UNSUPPORTED_AUTHORED_DATA`, lists at most twenty `details.findings` and says how many more
`details.finding_count` holds; any other refusal shows the API's own sentence. `review_wording` is
pure: instants as UTC lines, digests by their first twelve digits in headlines, unknown states as
the API spells them, and comments trimmed to 1 to 8000 bytes, as the API checks them; text outside
that bound is refused with `Error::InvalidReviewText`, whose sentence is the line shown under the
text box.

The wording, the refusal reading and the history and provenance views compile on every target and
are tested natively. `comment_composer`, `submission_action` and the `MissionReviewRecord`
component call the review endpoints, which exist only in the browser build, so they are compiled
for `wasm32` only.

## Boundaries

- Depends on: `frontend_session` (the signed-in `AuthStore` the browser calls read),
  `frontend_api_dtos` (the review wire types and identifiers), `frontend_transport` (the `mission_reviews` endpoint helpers, which call
  `GET /api/v1/missions/{id}/reviews` for `MissionReviewHistory`,
  `POST /api/v1/missions/{id}/review-comments` with `ReviewCommentRequest` for `ReviewComment`, and
  `POST /api/v1/missions/{id}/submit` for `MissionRow`; `Error`; `encode_path_segment`),
  `frontend_ui` (`MaterialIcon`, the toasts) and `frontend_ui::utc_timestamp`.
- Used by: in the mission hub, the overview page (`MissionReviewRecord`) and the library's dossier
  (`MissionReviewRecord`, `SubmitForReview`); the Mission Creator's review workspace banner
  (`short_digest`, `diagnostics_list`, `crates/frontend/workspaces/mission_creator_workspace/src/review_workspace/`); the [approvals](/documentation/glossary/a_to_f.md#approvals) page's review
  drawer, decision form and submission queue
  (`crates/frontend/pages/administration_pages/src/approvals/`); and the
  [server control](/documentation/glossary/n_to_z.md#server-control) page's
  [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) wording, which reads
  `short_digest` (`crates/frontend/pages/administration_pages/src/server_control/`).
- Rules: one rendering of a review, a thread entry and a compile finding for every page; the
  count a refusal reports is never below the findings it lists
  (`a_count_never_undercuts_the_listed_findings` in `tests/submission_refusal.rs`); an unknown
  refusal keeps the API's sentence (`other_refusals_keep_the_backend_sentence`); review text is
  trimmed and bounded (`review_text_is_trimmed_and_bounded` in `tests/review_wording.rs`).

## Related documentation

- [Mission approvals page](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md)
  — the administrators' side of a review.
- [Mission overview page](/documentation/crates/frontend/pages/mission_hub_pages/overview/mission_overview_page.md)
  — the dossier the record sits under.
- [Mission library page](/documentation/crates/frontend/pages/mission_hub_pages/library/mission_library_page.md#managing-a-mission)
  — the dossier sheet's submit control and its refusals.
- [Review workspace page](/documentation/crates/frontend/workspaces/mission_creator_workspace/review_workspace/review_workspace_page.md)
  — the read-only workspace each review links.
