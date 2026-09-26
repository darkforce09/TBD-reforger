# Review workspace page

The [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) opened read-only on exactly
the version an [artifact](/documentation_v2/glossary/a_to_f.md#artifact) compiled from, so a reviewer or
the author inspects what was submitted with every editor tool and saves nothing.

## Contents

```text
apps/website/frontend/src/v2/pages/mission_hub/review_workspace/
├── banner.rs  the persistent banner: artifact and version, read-only notice, compile findings
├── mod.rs     the module tree; re-exports `ReviewWorkspacePage`
├── page.rs    the route component: the workspace read, review mode and the mounted editor
└── tests/     unit tests for the banner, the refusal sentences and review mode's hold on writes
```

## How it works

`ReviewWorkspacePage` renders inside `AuthGate`. The signed-in half reads `:id` and `:artifact_id`
once and fetches the workspace: the artifact and its version, which the
[API](/documentation_v2/glossary/a_to_f.md#api) verifies against the payload digest the artifact recorded
and serves only to the [mission](/documentation_v2/glossary/g_to_m.md#mission)'s author and administrators.
Only once it has arrived does the page open the editor's review mode (`review_mode::open` with a
`ReviewedVersion`) and mount `MissionEditorPage` with the banner over it, so the editor's boot
always finds the reviewed version instead of restoring a draft; leaving the route closes review
mode.

While review mode is open the editor, in
`apps/website/frontend/src/v2/apps/editor/shell/review_mode.rs`, writes nothing: no draft record, no
warm-session marker, no writer election, no version, no mission-row change, and no unsaved-work
prompt on leaving. The document is stamped with the row fields the artifact's compile read, and
edits stay in the tab's memory until it closes. The banner stays for as long as the workspace is
open, since the editor otherwise looks like the Mission Creator: it names the artifact by its short
digest and the version, states that nothing is saved, gives the mission title, compile time and
document digest, and lists the compile findings (rule, severity, subject, message) in a disclosure.
Links to this route come from the review record's history and from the
[approvals](/documentation_v2/glossary/a_to_f.md#approvals) drawer.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/missions/:id/artifacts/:artifact_id/workspace` | `ReviewWorkspacePage` | route tier `mission_maker`; a viewer below it is sent to `/missions/:id?role_notice=mission_maker`; the API serves the workspace only to the mission's author and administrators | full-bleed and chromeless: no sidebar or top bar |

## Data

- `GET /api/v1/missions/{id}/artifacts/{artifact_id}/workspace`: through `load_review_workspace`
  in `crate::v2::core::api::endpoints::mission_reviews`, read as `ReviewWorkspace` (the artifact
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

- Depends on: `crate::v2::core::api` (`ReviewWorkspace`, the `mission_reviews` endpoint helpers,
  `encode_path_segment`, `api_error_message`), `crate::v2::core::ui` (`AuthGate`, `MaterialIcon`),
  `crate::v2::core::utils::utc_timestamp`, `short_digest` and `diagnostics_list` from
  `apps/website/frontend/src/v2/pages/mission_hub/mission_review/`, and, from the Mission Creator in
  `apps/website/frontend/src/v2/apps/editor/`, `mission_editor::MissionEditorPage` and
  `shell::review_mode`: this page imports the editor app directly.
- Used by: the route in `apps/website/frontend/src/app_routes.rs`; `review_workspace_href` in the
  review record (`apps/website/frontend/src/v2/pages/mission_hub/mission_review/`) and in the
  approvals drawer (`apps/website/frontend/src/v2/pages/administration/approvals/`) links here.
- Rules: the editor mounts only after the workspace read, inside review mode
  (`the_route_opens_review_mode_around_the_editor` and `the_editor_opens_on_the_reviewed_version`
  in `tests/review_workspace.rs`); review mode withholds every write
  (`review_mode_withholds_every_write_while_open`); a refused read says why
  (`a_refused_workspace_says_why`).

## Related documentation

- [Review workspace page](/documentation_v2/website/frontend/pages/mission_hub/review_workspace/review_workspace_page.md)
  — the page's behaviour, the digest check behind it and its open work.
- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  editor this page opens read-only.
