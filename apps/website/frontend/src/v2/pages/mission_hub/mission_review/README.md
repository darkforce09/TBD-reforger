# Mission review record

One rendering of a [mission](/documentation_v2/glossary/g_to_m.md#mission)'s review, shared by every page
that shows it: the review history and thread with a reply box, an
[artifact](/documentation_v2/glossary/a_to_f.md#artifact)'s provenance and compile findings, the submit
control with its refusal panel, and the wording under all of them, so the author and the reviewer
read the same record in the same words.

## Contents

```text
apps/website/frontend/src/v2/pages/mission_hub/mission_review/
├── artifact_provenance_view.rs  an artifact's provenance rows and the list of its compile findings
├── comment_composer.rs          `ReviewCommentComposer`: the reply box that posts a thread comment
├── mod.rs                       the module tree
├── review_history_view.rs       every review, newest first, then the thread in writing order
├── review_record.rs             `MissionReviewRecord`: history read, approved artifact and reply box
├── review_wording.rs            the pure wording: states and tones, digests, sizes, findings, links
├── submission_action.rs         `SubmitForReview`: the submit button and the refusal panel under it
├── submission_refusal.rs        a refused submission read into its reason and its listed findings
└── tests/                       unit tests for the review wording and the submission refusals
```

## How it works

`MissionReviewRecord` shows under the mission overview's dossier and in the library's dossier
sheet, for the author and administrators, the two the [API](/documentation_v2/glossary/a_to_f.md#api)
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
the API spells them, and comments trimmed to 1 to 8000 bytes, as the API checks them.

## Boundaries

- Depends on: `crate::v2::core::api` (the `mission_reviews` endpoint helpers, which call
  `GET /api/v1/missions/{id}/reviews` for `MissionReviewHistory`,
  `POST /api/v1/missions/{id}/review-comments` with `ReviewCommentRequest` for `ReviewComment`, and
  `POST /api/v1/missions/{id}/submit` for `MissionRow`; `ApiRefusal`; `encode_path_segment`),
  `crate::v2::core::ui` (`MaterialIcon`, the toasts) and `crate::v2::core::utils::utc_timestamp`.
- Used by: in the mission hub, the overview page (`MissionReviewRecord`), the library's dossier
  (`MissionReviewRecord`, `SubmitForReview`) and the review workspace banner (`short_digest`,
  `diagnostics_list`); the [approvals](/documentation_v2/glossary/a_to_f.md#approvals) page's review
  drawer, decision form and submission queue
  (`apps/website/frontend/src/v2/pages/administration/approvals/`); and the
  [server control](/documentation_v2/glossary/n_to_z.md#server-control) page's
  [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) wording, which reads
  `short_digest` (`apps/website/frontend/src/v2/pages/administration/server_control/`).
- Rules: one rendering of a review, a thread entry and a compile finding for every page; the
  count a refusal reports is never below the findings it lists
  (`a_count_never_undercuts_the_listed_findings` in `tests/submission_refusal.rs`); an unknown
  refusal keeps the API's sentence (`other_refusals_keep_the_backend_sentence`); review text is
  trimmed and bounded (`review_text_is_trimmed_and_bounded` in `tests/review_wording.rs`).

## Related documentation

- [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
  — the administrators' side of a review.
- [Mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md)
  — the dossier the record sits under.
- [Mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md#managing-a-mission)
  — the dossier sheet's submit control and its refusals.
- [Review workspace page](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md)
  — the read-only workspace each review links.
