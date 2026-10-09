# Mission review record

The `mission_review_record` crate: one rendering of a mission's review, shared by every page that
shows it — the review history and thread with a reply box, an artifact's provenance and compile
findings, the submit control with its refusal panel, and the wording under all of them — so the
author and the reviewer read the same record in the same words.

## Contents

```text
crates/frontend/features/mission_review_record/
├── Cargo.toml  the package: `frontend_session`, `frontend_transport`, `frontend_api_dtos`, `frontend_ui`, `leptos`, `serde_json`, `thiserror`, layout tier 10
└── src/        the review record, the submit control and its refusal reading, the history, thread and provenance views, the wording
```

## How it works

The mission hub's overview page and library dossier mount `review_record::MissionReviewRecord`,
which reads the mission's review history and renders the approved artifact, every review, the
thread and a reply box (`comment_composer::ReviewCommentComposer`). The library dossier's Manage
row mounts `submission_action::SubmitForReview`, whose refused answer `submission_refusal` reads
into the reason the backend names and the findings it lists. The approvals drawer and the review
workspace banner reuse the history, thread and provenance views and the wording directly. The
[source tree README](src/README.md) walks through each file.

The components that call the review endpoints are compiled for `wasm32` only, because those calls
exist only in the browser build; the wording, the refusal reading and the history and provenance
views compile on every target, so their tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_review_record   # the review wording against the captured goldens, the submission refusals
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `review_record`: `MissionReviewRecord` (wasm32) and `approved_artifact_line`.
- `submission_action`: `SubmitForReview` (wasm32); `submission_refusal`: `SubmissionRefusal` and
  `RefusalFindings`.
- `comment_composer`: `ReviewCommentComposer` (wasm32).
- `review_history_view`: `review_list` and `review_thread`; `artifact_provenance_view`:
  `artifact_provenance`, `diagnostics_list` and `provenance_rows`.
- `review_wording`: the pure wording — state labels and tones, digests, document sizes, findings,
  the review workspace link and `validated_review_text`.
- `error`: `Error` and `Result`, re-exported at the crate root.
- `prelude`: the components, views and wording most pages name.

## Boundaries

- Depends on: `frontend_session`, `frontend_transport`, `frontend_api_dtos`, `frontend_ui`,
  `leptos`, `serde_json`, `thiserror`; `frontend_test_support` for its tests only.
- Used by: the single-page app (`crates/frontend/shell/frontend_application`): the mission hub's overview and library dossier,
  the approvals drawer, decision form and submission queue, the server control page's mission
  deployment wording, and the Mission Creator's read-only review workspace banner.
- Rules: a feature crate depends on foundation crates only, never on a page or a workspace
  (`cargo xtask ci verify-workspace-laws`); every page renders a review, a thread entry and a
  compile finding through this crate.

## Related documentation

- [Source tree](src/README.md) — each file, the refusal codes and the tests that pin them.
- [Mission approvals page](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md)
  — the administrators' side of a review.
- [Mission overview page](/documentation/crates/frontend/pages/mission_hub_pages/overview/mission_overview_page.md)
  — the dossier the record sits under.
