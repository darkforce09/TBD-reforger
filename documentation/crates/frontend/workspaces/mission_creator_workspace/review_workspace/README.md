**Status:** live

# Review workspace page documentation

The feature documentation of the `/missions/:id/artifacts/:artifact_id/workspace` page, where the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) opens read-only on the version an
[artifact](/documentation/glossary/a_to_f.md#artifact) compiled from.

## Contents

```text
documentation/crates/frontend/workspaces/mission_creator_workspace/review_workspace/
└── review_workspace_page.md  the feature doc: opening the workspace, review mode and the API
```

## Code

- [Review workspace page](/crates/frontend/workspaces/mission_creator_workspace/src/review_workspace/) —
  the route component `ReviewWorkspacePage` and its banner.
- [Mission Creator session](/crates/frontend/workspaces/mission_creator_session/src/) — `review_mode.rs`,
  the read-only state the page opens.
- [Missions domain](/crates/api/api_missions/src/) — the workspace route and its digest check.

## Boundaries

- Depends on: the feature doc template; the page code, the editor's review mode, the missions
  handlers and the ticket registry the feature doc is written from.
- Used by: the in-code READMEs of the review workspace, the Mission Creator and the mission hub,
  which link the feature doc; the mission hub pages README and the Mission Creator documentation
  README.
- Rules: the feature doc keeps its name, which those links use; the page has no design set of its
  own, since it draws the Mission Creator.

## Related documentation

- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) — the
  editor this page opens read-only.
- [Mission approvals page](/documentation/crates/frontend/pages/administration_pages/approvals/mission_approvals_page.md)
  — the reviewer's queue, whose drawer links here.
