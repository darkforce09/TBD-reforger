**Status:** live

# Audit logs page documentation

The feature documentation of the `/admin/audit` page, where administrators read the trail of
administrative actions and inspect its entries, with the page's design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/administration/audit_logs/
├── audit_logs_page.md  the feature doc: the paged trail, the filter, the inspector and the API
└── visual_references/  the design-phase blueprint of a terminal-style audit console
```

## How it works

Read [audit_logs_page.md](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what the list call means in the
[API](/documentation_v2/glossary/a_to_f.md#api) and which audit routes the page leaves unused, and compares
the built page with the blueprint in `visual_references/`. The blueprint is a design-phase
reference: it shows a live feed and a CSV export, which the built page does not offer. The code
folder's README lists the page's files.

## Code

- [Audit logs page](/apps/website/frontend/src/v2/pages/administration/audit_logs/) — the route
  component `AuditLogsPage`, the trail, the filter and the entry inspector.
- [Administration domain](/apps/website/api_v2/src/administration/) — the audit log routes and the
  service every state-changing handler records its entry through.

## Boundaries

- Depends on: the feature doc template; the page code, the audit log handlers and the ticket
  registry the feature doc is written from.
- Used by: the [audit logs](/documentation_v2/glossary/a_to_f.md#audit-logs) glossary entry, the page's
  in-code README and the administration domain README, which link the feature doc; the
  administration pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Administration domain](/apps/website/api_v2/src/administration/README.md) — the API side of the
  audit trail.
