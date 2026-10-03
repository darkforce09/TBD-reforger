**Status:** live

# Ballistics catalogs page documentation

The feature documentation of the `/admin/ballistics-catalogs` page, where administrators publish
a ballistics catalog version, the weapon and shell values the mortar calculator solves with, after
the [API](/documentation/glossary/a_to_f.md#api) has judged it against its calibration bundle.
The page has no design set: its built layout, described in the feature doc, is the reference.

## Contents

```text
documentation/apps/frontend/pages/administration/ballistics_catalogs/
└── ballistics_catalogs_page.md  the feature doc: the upload, the validation report and the stored versions
```

## Code

- [Ballistics catalogs page](/apps/frontend/src/pages/administration/ballistics_catalogs/)
  — the route component `BallisticsCatalogsPage`, the upload form, the validation report and the
  version list.
- [Operations domain](/apps/api/src/operations/) — the catalog upload and read routes.
- [Game ballistics](/crates/ballistics/README.md) — the calibration the
  upload runs.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  page code, the operations catalog handlers and the `ballistics_calibration` crate the feature doc is
  written from.
- Used by: the administration pages README; the mortar calculator and game ballistics engine
  feature docs, which link the feature doc.
- Rules: the feature doc keeps its name, which those links use, and stays within 500 lines.

## Related documentation

- [Game ballistics design note](/documentation/apps/api/verification_evidence/game_ballistics.md)
  — the upload lifecycle, the calibration criterion and the fixtures.
- [Ballistics oracle run](/documentation/runbooks/ballistics_oracle_run.md) — producing a new
  catalog version and its calibration bundle.
