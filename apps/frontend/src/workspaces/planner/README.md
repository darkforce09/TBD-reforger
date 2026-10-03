# Mission planner workspace

The folder reserved for the mission planner, a tactical whiteboard on which a briefing draws its
plan over a published [mission](/documentation/glossary/g_to_m.md#mission) without editing it. It
holds no code: only this README.

## Contents

```text
apps/frontend/src/workspaces/planner/
```

## Routes

None: the folder serves no route, and `apps/frontend/src/app_routes.rs` has no planner
route.

## Public surface

None: `apps/frontend/src/workspaces/mod.rs` declares no `planner` module, so nothing here
compiles.

## Boundaries

- Depends on: nothing.
- Used by: nothing.
- Rules: a workspace added here follows the rules of
  `apps/frontend/src/workspaces/`: it declares its module in that folder's `mod.rs`, imports
  from `crate::foundation` and `map_engine`, and never from a page or a sibling workspace;
  its route needs a row in both `apps/frontend/src/app_routes.rs` and
  `apps/frontend/src/foundation/route_table/mod.rs`.

## Related documentation

- [Mission planner](/documentation/apps/frontend/workspaces/planner/mission_planner.md) — the planned workspace's design notes and open work.
