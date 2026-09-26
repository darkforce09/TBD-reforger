**Status:** live

# Review workspace page documentation

The feature documentation of the `/missions/:id/artifacts/:artifact_id/workspace` page, where the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) opens read-only on the version an
[artifact](/documentation_v2/glossary/a_to_f.md#artifact) compiled from.

## Contents

```text
documentation_v2/website/frontend/pages/mission_hub/review_workspace/
└── review_workspace_page.md  the feature doc: opening the workspace, review mode and the API
```

## Code

- [Review workspace page](/apps/website/frontend/src/v2/pages/mission_hub/review_workspace/) —
  the route component `ReviewWorkspacePage` and its banner.
- [Mission Creator shell](/apps/website/frontend/src/v2/apps/editor/shell/) — `review_mode.rs`,
  the read-only state the page opens.
- [Missions domain](/apps/website/api_v2/src/missions/) — the workspace route and its digest check.

## Boundaries

- Depends on: the feature doc template; the page code, the editor's review mode, the missions
  handlers and the ticket registry the feature doc is written from.
- Used by: the in-code READMEs of the review workspace and the mission hub, which link the feature
  doc; the mission hub pages README.
- Rules: the feature doc keeps its name, which those links use; the page has no design set of its
  own, since it draws the Mission Creator.

## Related documentation

- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  editor this page opens read-only.
- [Mission approvals page](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
  — the reviewer's queue, whose drawer links here.
