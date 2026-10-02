# Wiki saves and restores

The wiki's two writes — an administrator's draft save and a restore of an older revision — from the
request body, through `PUT /wiki/{slug}`, to the notice that reports a refusal.

## Contents

```text
apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/saving/
├── mod.rs                declares the saving modules and re-exports what the panes call
├── save_problem_view.rs  the refusal alert: headline, one line per refused construct, conflict reload
├── save_refusal.rs       `SaveProblem` and `SaveFailure`: a refusal classified and worded
├── save_requests.rs      `draft_save_request` and `restore_request`: the two `WikiSaveRequest` bodies
├── save_submission.rs    `submit_save`: the `PUT`, and the answer applied to the page
└── tests/                the request bodies and the refusal mapping
```

## How it works

`draft_save_request` sends the draft (or the stored body) with the revision the draft started
from, plus the article's stored category, title, icon and navigation order; `restore_request`
sends every field of an older revision over the article's current revision. Both carry a numeric
`base_revision`: the page never creates a manual.

`submit_save` refuses a second write while one is in flight, then sends the body with
`api_put_keeping_refusal`. An accepted save drops the draft (a draft save) or leaves the revision
view (a restore), returns the pane to reading, bumps the article's reload counters, refetches the
page list and raises a notice. A refusal becomes a `SaveFailure`: `SaveProblem::from_refusal`
reads the `details` as a `WikiSaveRefusal` when they parse, and the status otherwise.

| Answer | Problem | What the author sees |
|---|---|---|
| 409, or `wiki_revision_conflict` | `RevisionConflict` | both revision numbers, and "Discard my draft and load revision <n>" ("Load revision <n>" after a restore) |
| 422 `wiki_markup_refused` | `MarkupRefused` | "Not saved: the markup has <k> problems." and "Line <l>: <detail>" per finding |
| 400 `wiki_body_too_large` | `BodyTooLarge` | the 262 144-byte limit and the advice to split the manual |
| 413 | `RequestTooLarge` | "Not saved: the request is larger than the server accepts." |
| anything else | `Refused` | the unreached, expired-session and administrator-only sentences, or the API's own sentence |

The conflict's reload goes through `ArticleState::reload_after_conflict`: a refused draft save
drops the draft so the editor opens on the latest text; a refused restore keeps any draft.

## Boundaries

- Depends on: `../article/` (`ArticleState`), `../page_state.rs` (`WikiPageState`, `WikiDraft`,
  `WikiMode`), `../api_paths.rs`, `crate::v2::core::api` (`api_put_keeping_refusal`,
  `ApiRefusal`, `dto::wiki`) and `crate::v2::core::ui::toast`.
- Used by: `../article/article_header.rs` (the draft save), `../article/article_pane.rs` (the
  refusal alert), `../article/article_state.rs` (the failure types) and
  `../revisions/revision_view.rs` (the restore).
- Rules: every save names the revision its edit started from
  (`wiki_save_request_sends_the_draft_with_the_revision_it_started_from`); a restore saves over the
  current revision (`wiki_restore_request_sends_the_old_revision_over_the_current_one`); only a
  conflict offers a reload (`wiki_refusal_markup_lists_every_finding_with_its_line`); the request
  runs on `wasm32` only.

## Related documentation

- [Doctrine wiki page](/documentation/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md)
  — editing, conflicts and restores.
- [Administration and community content](/documentation/website/api_v2/verification_evidence/administration_and_content.md#saving)
  — the server's save rules and refusals.
