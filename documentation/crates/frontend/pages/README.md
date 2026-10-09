**Status:** live

# Page crate documentation

The feature documentation of the page crates under `crates/frontend/pages/`: one folder per
crate, and in it one folder per page with the page's feature doc and, where a design set exists,
its design references. Developers and AI agents read it before changing a page.

## Contents

```text
documentation/crates/frontend/pages/
├── account_pages/  the sign-in, sign-in callback and account settings pages
├── administration_pages/  the event manager, approvals, server control, personnel, content, audit logs and ballistics catalogs pages
├── command_center_pages/  the dashboard, server intel and announcements pages
├── doctrine_pages/  the doctrine wiki, vehicle database and modpacks pages
├── field_tools_pages/  the mortar calculator page
├── mission_hub_pages/  the mission library and mission overview pages
└── operations_pages/  the event schedule, event hub, ORBAT selection, deployments and leaderboards pages
```

## Code

- [Frontend page crates](/crates/frontend/pages/README.md) — the crates the documents below
  describe.

## Boundaries

- Depends on: the code of `crates/frontend/pages/`; the
  [feature doc template](/documentation/standards/templates/feature_doc.md) and the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md).
- Used by: the [frontend crate documentation](/documentation/crates/frontend/README.md) index; the
  page folders' READMEs, which link their feature docs.
- Rules: one folder per page crate, spelled like the crate; below it the folders mirror the
  crate's `src/` page folders.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) — every route with its code
  folder and feature doc.
