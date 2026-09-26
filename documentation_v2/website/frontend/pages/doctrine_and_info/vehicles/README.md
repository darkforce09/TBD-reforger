**Status:** live

# Vehicle database page documentation

The feature documentation of the `/vehicles` page, where members look up vehicles by faction and
read one vehicle's identification dossier, with the page's two design-phase references.

## Contents

```text
documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/
├── vehicle_database_page.md  the feature doc: the faction list, the dossier and the API
└── visual_references/        the design-phase dossier blueprint and a top-down BTR-70 render
```

## How it works

Read [vehicle_database_page.md](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what the vehicle routes of the
[API](/documentation_v2/glossary/a_to_f.md#api) do, and compares the built page with the sets in
`visual_references/`. Both sets are design-phase references: the blueprint draws a dossier full of
speed, crew and armament figures and a manual sidebar, while the built dossier shows only the
armour class, amphibious value and threat a vehicle row stores. The code folder's README lists
the page's files.

## Code

- [Vehicle database page](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/) — the
  route component `VehicleDatabasePage`, the list and the dossier.
- [Community content domain](/apps/website/api_v2/src/community_content/) — the vehicle database
  routes.

## Boundaries

- Depends on: the feature doc template; the page code, the vehicle handlers and the ticket registry
  the feature doc is written from.
- Used by: the in-code READMEs of the vehicle database and of the doctrine and info pages, which
  link the feature doc; the doctrine and info pages README.
- Rules: the feature doc keeps its name, which those links use; the sets stay as they were
  captured and are never edited to match the built page.

## Related documentation

- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the API side
  of the vehicle rows.
