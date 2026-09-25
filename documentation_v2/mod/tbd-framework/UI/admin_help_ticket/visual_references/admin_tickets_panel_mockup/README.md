**Status:** live

# Admin tickets panel mockup

Design-phase reference for the admin menu's Tickets module: the queue of player help requests. It gives layout and colour context and is not an implementation source; no built UI exists
for it.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/admin_help_ticket/visual_references/admin_tickets_panel_mockup/
├── admin_tickets_panel_mockup.html  the Stitch export
└── admin_tickets_panel_mockup.png   its screenshot
```

## How it works

The set shows the admin header ("SERVER FPS: 120", "TIME: 11:30"), filters "All (4)", "Pending (2)" and "Resolved (1)", and ticket cards ("#1042 COLLISION PENDING" for "[1stID] Cpt. Miller" at "GRID: 042-088", "#1040 CAMERA" claimed by a referee, "#1039 VEHICLE RESOLVED"). The selected ticket, "STUCK IN MESH / COLLISION" with "ELEVATED PRIORITY", shows its submission time and origin, the player with "Teleport To" and "Spectate", grid, health ("BLEEDING"), posture ("WEDGED / TRAP") and transport, the player's description and quick tags, and the actions "Unstick to Surface", "Full Heal & Revive", "Fix Uniform/Vest", "Dispatch Toast to Player & Close" and "Resolve & Notify".

Nothing of the module is built: players have no way to file a ticket, and the admin screen has no tickets section. The [admin help ticket specification](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md) feature doc holds the design.

## Code

- [Admin](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/) — the admin code the module
  would join.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
