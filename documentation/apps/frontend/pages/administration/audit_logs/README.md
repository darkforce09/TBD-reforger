**Status:** live

# Audit logs page documentation

The feature documentation of the `/admin/audit` page, where administrators read the trail of
administrative actions and inspect its entries, with the page's design-phase reference.

## Contents

```text
documentation/apps/frontend/pages/administration/audit_logs/
├── audit_logs_page.md  the feature doc: the live and paged trail, the filter, the inspector and the API
└── visual_references/  the design-phase blueprint of a terminal-style audit console
```

## How it works

Read [audit_logs_page.md](/documentation/apps/frontend/pages/administration/audit_logs/audit_logs_page.md)
first. It follows the [feature doc template](/documentation/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what the stream and the list calls mean in the
[API](/documentation/glossary/a_to_f.md#api) and which audit route the page leaves unused, and
compares the built page with the blueprint in `visual_references/`. The blueprint is a design-phase
reference: it shows a "Live Feed" badge and a CSV export; the built page streams new entries under
its own status badge and offers no export. The code
folder's README lists the page's files.

## Code

- [Audit logs page](/apps/frontend/src/pages/administration/audit_logs/) — the route
  component `AuditLogsPage`, the trail, the filter and the entry inspector.
- [Administration domain](/crates/api/api_administration/src/) — the audit log routes and the
  service every state-changing handler records its entry through.

## Boundaries

- Depends on: the feature doc template; the page code, the audit log handlers and the ticket
  registry the feature doc is written from.
- Used by: the [audit logs](/documentation/glossary/a_to_f.md#audit-logs) glossary entry, the page's
  in-code README and the administration domain README, which link the feature doc; the
  administration pages README.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Administration domain](/crates/api/api_administration/src/README.md) — the API side of the
  audit trail.
