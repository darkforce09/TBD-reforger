**Status:** live

# Service record blueprint

Design-phase reference for the deployments page at `/deployments`: a member's
[service record](/documentation_v2/glossary.md#service-record), with career figures beside the
next operation's orders and a combat history. It gives colour and layout context and is not an
implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/operations/deployments/`.

## Contents

```text
documentation_v2/website/frontend/pages/operations/deployments/visual_references/service_record_blueprint/
├── service_record_blueprint.html  the Stitch export of the service record
└── service_record_blueprint.png   its screenshot
```

## How it works

The blueprint shows a left column with a rank and name, a "K/D RATIO", a "WIN RATE", "TOTAL
DEPLOYMENTS", a "FAV WEAPON" and a "FAV ASSET"; and a right column with "ACTIVE ORDERS" (an
operation's name, "ROLE:" with a callsign and role, "T-MINUS 24:00:00" and a "MODIFY ASSIGNMENT"
button) over "COMBAT HISTORY", a timeline of past operations, each with a date, a role, a K/D and a
"VICTORY" or "DEFEAT" badge.

The built page keeps the two columns and differs: the left column shows the name, the
[role](/documentation_v2/glossary.md#role), the deployment count and "No telemetry recorded", with
no K/D, win rate or favourites; the banner adds reservation and attendance badges, a terrain and an
"Operation Hub" link, and counts down in one rounded unit; the history is a table with an outcome
and a replay link instead of a timeline with K/D; and a leave panel and an administrator's review
queue follow. The
[deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md)
feature doc holds the full comparison.

## Code

- [Deployments page](/apps/website/frontend/src/v2/pages/operations/deployments/) — the page this
  set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and Google-hosted images, which the html loads
  when opened; the png needs nothing.
- Used by: the deployments feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
