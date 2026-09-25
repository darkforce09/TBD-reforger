**Status:** live

# Command dashboard blueprint

Design-phase reference for the dashboard page at `/`: a countdown banner over three status cards
and an intelligence feed. It gives colour and layout context and is not an implementation source;
the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/command_center/dashboard/`.

## Contents

```text
documentation_v2/website/frontend/pages/command_center/dashboard/visual_references/command_dashboard_blueprint/
├── command_dashboard_blueprint.html  the Stitch export of the dashboard
└── command_dashboard_blueprint.png   its screenshot
```

## How it works

The blueprint shows a banner over a satellite map with "T-MINUS 04:12:30",
"OPERATION: FIRESTORM — EVERON" and an "ENTER INTELLIGENCE HUB" button; a "SERVER UPLINK" card
with an "ONLINE" pill, "45/64" players and a fill bar, "PING: 24ms" and "LOC: EU-WEST-1"; a
"DEPLOYMENT" card with a faction, a squad and "ROLE: RIFLEMAN"; a "MODPACK" card with a name and
version, "SIZE: 14.2 GB" and "STATUS: SYNCED"; and "RECENT INTELLIGENCE", four timestamped
messages with a headline and a line of detail.

The built page keeps the layout and the headings and differs: the countdown is one rounded unit
("T-MINUS 3 HOURS") fixed at render; the button reads "Open Operation Hub"; the uplink card shows
frame rate and uptime instead of ping and location; the feed lists the three newest
announcements with a date pill, a pin icon and a preview, where the blueprint shows operational
messages with times. The
[dashboard page](/documentation_v2/website/frontend/pages/command_center/dashboard/dashboard_page.md)
feature doc holds the full comparison.

## Code

- [Dashboard page](/apps/website/frontend/src/v2/pages/command_center/dashboard/) — the page this
  set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and a Google-hosted image, which the html loads
  when opened; the png needs nothing.
- Used by: the dashboard feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
