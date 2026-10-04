# Review workspace page

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) opened read-only on exactly
the version an [artifact](/documentation/glossary/a_to_f.md#artifact) compiled from, so a reviewer or
the author inspects what was submitted with every editor tool and saves nothing.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/review_workspace/
├── banner.rs  the persistent banner: artifact and version, read-only notice, compile findings
├── mod.rs     the module tree; re-exports `ReviewWorkspacePage`
├── page.rs    the route component: the workspace read, review mode and the mounted editor
└── tests/     unit tests for the banner, the refusal sentences and review mode's hold on writes
```

## How it works

`ReviewWorkspacePage` renders inside `AuthGate`. The signed-in half reads `:id` and `:artifact_id`
once and fetches the workspace: the artifact and its version, which the
[API](/documentation/glossary/a_to_f.md#api) verifies against the payload digest the artifact recorded
and serves only to the [mission](/documentation/glossary/g_to_m.md#mission)'s author and administrators.
Only once it has arrived does the page open the editor's review mode (`review_mode::open` with a
`ReviewedVersion`) and mount `MissionEditorPage` with the banner over it, so the editor's boot
always finds the reviewed version instead of restoring a draft; leaving the route closes review
mode.

While review mode is open the editor, in
`crates/frontend/workspaces/mission_creator_state/src/review_mode.rs`, writes nothing: no draft record, no
warm-session marker, no writer election, no version, no mission-row change, and no unsaved-work
prompt on leaving. The document is stamped with the row fields the artifact's compile read, and
edits stay in the tab's memory until it closes. The banner stays for as long as the workspace is
open, since the editor otherwise looks like the Mission Creator: it names the artifact by its short
digest and the version, states that nothing is saved, gives the mission title, compile time and
document digest, and lists the compile findings (rule, severity, subject, message) in a disclosure.
Links to this route come from the review record's history and from the
[approvals](/documentation/glossary/a_to_f.md#approvals) drawer.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` | route tier `mission_maker`; a viewer below it is sent to `/missions/:id?role_notice=mission_maker`; the API serves the workspace only to the mission's author and administrators | full-bleed and chromeless: no sidebar or top bar |

## Data

- `GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace`: through `load_review_workspace`
  in `frontend_transport::endpoints::mission_reviews`, read as `ReviewWorkspace` (the artifact
  with its metadata and compile diagnostics, and the version with its `semver` and payload).
- The page reads the session from the `AuthStore` context and the path parameters from the router;
  it writes only the editor's in-memory review mode cell. The read runs in the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Opening the review workspace…" |
| loaded | the editor with the banner "Review workspace of artifact <short digest>, version <semver>", "Read-only: nothing here is saved — no draft, no version, no submission, no mission settings. Edits stay in this tab and are discarded when it closes.", "<title> · compiled <UTC time> · document <digest>", "The compile reported N findings" (or "no findings", "1 finding") and "Back to the mission" |
| session ended (401) | "Your session has ended — sign in again to open the review workspace." and "Back to the mission" |
| refused (403) | "Only the mission's author and administrators can open its review workspace." and "Back to the mission" |
| not found (404) | "There is no such artifact of this mission, or the mission is not visible to you." and "Back to the mission" |
| other failure | the API's message, or "The review workspace could not be opened", and "Back to the mission" |

## Boundaries

- Depends on: `frontend_transport` (`ReviewWorkspace`, the `mission_reviews` endpoint helpers,
  `encode_path_segment`, `Error::message_or`), `frontend_ui` (`AuthGate`, `MaterialIcon`),
  `frontend_ui::utc_timestamp`, `short_digest` and `diagnostics_list` from
  `crates/frontend/features/mission_review_record/src/`, and, from its own workspace
  `crates/frontend/workspaces/mission_creator_workspace/src/`, `mission_editor::MissionEditorPage`,
  `mission_creator_state::review_mode` and, in its tests, `mission_creator_session::tab_lock`.
- Used by: the route in `apps/frontend/src/app_routes.rs`; `review_workspace_href` in the
  review record (`crates/frontend/features/mission_review_record/src/`) and in the
  approvals drawer (`crates/frontend/pages/administration_pages/src/approvals/`) links here.
- Rules: the editor mounts only after the workspace read, inside review mode
  (`the_route_opens_review_mode_around_the_editor` and `the_editor_opens_on_the_reviewed_version`
  in `tests/review_workspace.rs`); review mode withholds every write
  (`review_mode_withholds_every_write_while_open`); a refused read says why
  (`a_refused_workspace_says_why`).

## Related documentation

- [Review workspace page](/documentation/crates/frontend/workspaces/mission_creator_workspace/review_workspace/review_workspace_page.md)
  — the page's behaviour, the digest check behind it and its open work.
- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) — the
  editor this page opens read-only.
