**Status:** live

# Audit logs blueprint

Design-phase reference for the audit logs page at `/admin/audit`: a terminal-style console of
the system log. It gives colour and layout context and is not an implementation source; the built
UI is the Leptos code under `apps/website/frontend/src/v2/pages/administration/audit_logs/`.

## Contents

```text
documentation_v2/website/frontend/pages/administration/audit_logs/visual_references/audit_logs_blueprint/
├── audit_logs_blueprint.html  the Stitch export of the audit console
└── audit_logs_blueprint.png   its screenshot
```

## How it works

The blueprint shows the heading "System Audit Logs" with the line "Real-time terminal view of all
system events and operations.", a filter field ("Filter by admin, event, or keyword..."), an
"Export to CSV" button, and a terminal window with traffic-light dots, a `~/syslog_view` title
and a "Live Feed" badge, holding `[timestamp] [LEVEL] message` lines. Beside it an "Incident
Details" panel shows the stack trace and the JSON payload of a critical entry.

The built page differs: no heading of its own, no CSV button, no live feed and no terminal
chrome; each line adds the action; the filter reads "Filter by admin, action, or keyword..." and
filters only what is loaded; and the inspector shows an entry's fields and metadata, with no
stack trace. The
[audit logs page](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md)
feature doc holds the full comparison.

## Code

- [Audit logs page](/apps/website/frontend/src/v2/pages/administration/audit_logs/) — the page
  this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the audit logs feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
