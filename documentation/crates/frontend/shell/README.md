**Status:** live

# Shell crate documentation

The feature documentation of the shell crates under `crates/frontend/shell/`: one folder per
crate that has documents beyond its code READMEs. The single-page app's folder is the
documentation hub of the whole app, with every route, and holds its frame's feature docs; the
offline service worker has no folder, since its code READMEs describe it whole. Developers and AI
agents start here before changing a page.

## Contents

```text
documentation/crates/frontend/shell/
└── frontend_application/  the single-page app: the documentation hub with the route table, and the app frame's feature docs
```

## Code

- [Frontend shell crates](/crates/frontend/shell/README.md) — the crates the documents below
  describe.

## Boundaries

- Depends on: the code of `crates/frontend/shell/`; the
  [feature doc template](/documentation/standards/templates/feature_doc.md) and the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md).
- Used by: the [frontend crate documentation](/documentation/crates/frontend/README.md) index.
- Rules: one folder per shell crate with documents of its own, spelled like the crate; below it the
  folders mirror the crate's `src/` folders.

## Related documentation

- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) —
  every route with its code folder and feature doc.
