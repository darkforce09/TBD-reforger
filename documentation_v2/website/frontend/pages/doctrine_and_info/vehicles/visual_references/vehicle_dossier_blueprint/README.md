**Status:** live

# Vehicle dossier blueprint

Design-phase reference for the vehicle database page at `/vehicles`: a faction-grouped vehicle
list beside a BTR-70 identification dossier, inside a wiki-style frame. It gives colour and layout
context and is not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/`.

## Contents

```text
documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/visual_references/vehicle_dossier_blueprint/
├── vehicle_dossier_blueprint.html  the Stitch export of the list and the dossier
└── vehicle_dossier_blueprint.png   its screenshot
```

## How it works

The blueprint shows a "TACTICAL SOP WIKI" frame with a sidebar of manual sections ("Core
Procedures", "Vehicle Ops", "Communications", "Medical SOP" and more) and the actions "Publish
Revision", "Archived" and "Export PDF"; a list flagged by faction ("US FORCES" with the LAV-25 and
M1A1 Abrams, "USSR FORCES" with the BTR-70, BMP-2 and T-72A); and a dossier with "CLASS: APC",
"THREAT: MED", a description, a "CRITICAL:" identification warning, "Technical Telemetry"
(mobility, defence, capacity), "Armament" and "Primary Threats To Target" icons.

The built page differs: no manual sidebar and no revision, archive or export actions; the list is
one searchable column grouped by the stored faction, without flags; the dossier shows the stored
armour class, amphibious value and primary threat, and "Telemetry" repeats the faction, armour and
amphibious value, since a vehicle row stores no description, speeds, crew, armament or threat
icons. The
[vehicle database page](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md)
feature doc holds the full comparison.

## Code

- [Vehicle database page](/apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/) — the
  page this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and Google-hosted placeholder images, which the
  html loads when opened; the png needs nothing.
- Used by: the vehicle database feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
