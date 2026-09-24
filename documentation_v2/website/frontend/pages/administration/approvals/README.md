**Status:** live

# Mission approvals page documentation

The feature documentation of the `/admin/approvals` page, where administrators review the
[artifacts](/documentation_v2/glossary.md#artifact) that submitted
[missions](/documentation_v2/glossary.md#mission) compiled into and decide them, with the page's
design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/administration/approvals/
├── mission_approvals_page.md  the feature doc: the queue, the review drawer and the decision rules
└── visual_references/         the design-phase blueprint of a mission review dossier
```

## How it works

Read [mission_approvals_page.md](/documentation_v2/website/frontend/pages/administration/approvals/mission_approvals_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what each call means in the
[API](/documentation_v2/glossary.md#api), and compares the built page with the blueprint in
`visual_references/`. The blueprint is a design-phase reference: it shows a mission dossier with a
map and side counts, while the built page reviews an immutable artifact with its provenance and
compile findings. The code folder's README lists the page's files.

## Code

- [Mission approvals page](/apps/website/frontend/src/v2/pages/administration/approvals/) — the
  route component `MissionApprovalsPage`, the queue, the review drawer and the decision form.
- [Mission review views](/apps/website/frontend/src/v2/pages/mission_hub/mission_review/) — the
  provenance, review record and comment views the drawer shares with the mission hub.
- [Missions domain](/apps/website/api_v2/src/missions/) — the approvals queue, the review record
  and the decisions.

## Boundaries

- Depends on: the feature doc template; the page code, the mission review views and the missions
  domain handlers the feature doc is written from.
- Used by: the [approvals](/documentation_v2/glossary.md#approvals) glossary entry and the page's
  in-code README, which link the feature doc; the administration pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Mission artifacts evidence](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — the verification of artifacts, reviews and deployments that the review drawer relies on.
