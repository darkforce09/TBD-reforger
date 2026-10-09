**Status:** live

# Frontend crate documentation

The feature documentation of the single-page app's crates under `crates/frontend/`, one folder per
layer and below it one per crate, for the app, the pages and the workspaces whose behaviour, data
and design outgrow their code READMEs. Developers and AI agents read it below those READMEs.

## Contents

```text
documentation/crates/frontend/
├── pages/       the platform pages: one folder per page crate, one feature doc per page
├── shell/       the app itself: the documentation hub of the single-page app and its frame's feature docs
└── workspaces/  the full-screen applications: one folder per workspace crate, one feature doc per bench or tool, and the planned workspaces
```

## Code

- [Frontend crates](/crates/frontend/README.md) — the crates the documents below describe.

## Boundaries

- Depends on: the code of `crates/frontend/`; the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md).
- Used by: the [library crate documentation](/documentation/crates/README.md) index.
- Rules: code at `crates/frontend/<layer>/<crate>/src/<path>` is documented at
  `documentation/crates/frontend/<layer>/<crate>/<path>`; the folders keep the code's spelling,
  a planned workspace's folder under `workspaces/` excepted.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) —
  the app these crates serve, with each route's code folder and feature doc.
