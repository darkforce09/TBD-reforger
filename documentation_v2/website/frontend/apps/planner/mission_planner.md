**Status:** live

# Mission planner

A planned workspace, not built: a tactical whiteboard on which squad leaders and commanders plan an
[event](/documentation_v2/glossary.md#event)'s operation on the map of its published
[mission](/documentation_v2/glossary.md#mission), drawing markers and control measures over it
without editing the mission, and carry that plan into the game.

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/planner/`](/apps/website/frontend/src/v2/apps/planner/README.md),
  a reserved folder that holds only its README; `apps/website/frontend/src/v2/apps/mod.rs` declares
  no `planner` module.
- Entry: none. `apps/website/frontend/src/app_routes.rs` and `apps/website/frontend/src/router.rs`
  have no planner route.
- Related features: the [Mission Creator](/documentation_v2/website/frontend/apps/editor/README.md),
  whose map, briefing markers and tactical graphics the planner would draw on; the
  [event hub page](/documentation_v2/website/frontend/pages/operations/event_detail/event_hub_page.md),
  where an event's missions and [ORBAT](/documentation_v2/glossary.md#orbat) are read today.

## Behaviour

None: no code serves the planner. The nearest built pieces are these:

1. The [Mission Creator](/documentation_v2/glossary.md#mission-creator) lets the mission maker drop
   briefing markers for one side and draw tactical graphics (phase lines, boundaries, axes of
   advance and curved arrows) into the mission itself; the checks live in
   `apps/website/map-engine/src/data/scenario/extensions/tactical_graphics/`.
2. The [mod](/documentation_v2/glossary.md#mod) puts a mission's briefing markers on the in-game
   map and sends each player only their own side's markers
   (`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/`).
3. Nothing stores a plan per member or per event, and no in-game menu loads a plan from the
   website.

## Data

None today: the API has no planner route and the database no plan table. The design needs a
per-member plan store bound to an event mission, a way to share a commander's plan with a side,
and a read the game server can make for the player who asks.

## Design

The design is at the idea stage, drawn from the product blueprint (archived at
`documentation_v2/archive/go_and_react_era_design/mission_creator_design.md`, section 3) and the
planner's first design draft. No visual reference set exists.

### Design notes

- The planner uses the same 2D map as the Mission Creator, read-only: the plan is drawn over a
  published mission and never changes it.
- Players draw tactical markers as in the Arma 3 SWT markers mod: lines, arrows, phase lines,
  objective markers, boundaries and ingress and egress routes. The draft names NATO
  MIL-STD-2525 tactical graphics; the map engine's symbology today is its own unit-role and vehicle
  glyph set (`apps/website/map-engine/src/overlay/symbology/`), not a 2525 set.
- Markup is collaborative and live between the planners of one event.
- A briefing view pairs the plan with slides and a squad assignment dossier.
- Identity sync: because a member's Discord account is linked to their Arma identity, "Save Plan
  to Profile" stores the plan against the member.
- Opt-in loading in the game: the briefing map carries a "Load Web Plan" button (the blueprint
  also calls it "Import Web Plan") that fetches the player's own markers, or a commander's shared
  ones, and plots them. Loading is never automatic, so ten squad leaders' plans do not pile onto
  one map.

## Open work

None: no ticket in `.ai/tickets/` covers the planner workspace. T-131, "Route planner tool"
(ready), is a Mission Creator tool that snaps routes to roads, not this workspace.

## Decisions

- The folder is reserved and holds no code until the workspace is built: a workspace is added with
  its module line, its route and its README together, as the
  [workspaces README](/apps/website/frontend/src/v2/apps/README.md) rules require.
- A plan never edits the mission: planners draw over a published mission, the mission maker's
  document stays as it was reviewed, and each planner's markup sits beside it.
