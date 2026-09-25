**Status:** live

# Deployments page documentation

The feature documentation of the `/deployments` page, labelled "My Deployments", where a member
reads their own [service record](/documentation_v2/glossary.md#service-record) and files leave,
and an administrator decides leave requests, with the page's design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/operations/deployments/
├── deployments_page.md  the feature doc: the record, upcoming deployments, leave and the API
└── visual_references/   the design-phase blueprint of a service record beside active orders
```

## How it works

Read [deployments_page.md](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what the record and the leave calls mean in the
[API](/documentation_v2/glossary.md#api), lists where the page and the API disagree, and compares
the built page with the blueprint in `visual_references/`. The code folder's README lists the
page's files, calls and states.

## Code

- [Deployments page](/apps/website/frontend/src/v2/pages/operations/deployments/) — the route
  component `DeploymentsPage`, the banner, the history table and the leave panels.
- [Operations domain](/apps/website/api_v2/src/operations/) — the service record and the leave
  requests.

## Boundaries

- Depends on: the feature doc template; the page code, the operations handlers and the ticket
  registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the page's in-code README, the operations pages README and the operations domain
  README, which link the feature doc; the [service record](/documentation_v2/glossary.md#service-record)
  glossary entry; the ORBAT selection feature doc; the web app README's page table in
  `documentation_v2/website/frontend/`.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Operations domain](/apps/website/api_v2/src/operations/README.md) — the API side of the
  service record and leave.
