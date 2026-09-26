# Mission planner workspace

The folder reserved for the mission planner, a tactical whiteboard on which a briefing draws its
plan over a published [mission](/documentation_v2/glossary/g_to_m.md#mission) without editing it. It
holds no code: only this README.

## Contents

```text
apps/website/frontend/src/v2/apps/planner/
```

## Routes

None: the folder serves no route, and `apps/website/frontend/src/app_routes.rs` has no planner
route.

## Public surface

None: `apps/website/frontend/src/v2/apps/mod.rs` declares no `planner` module, so nothing here
compiles.

## Boundaries

- Depends on: nothing.
- Used by: nothing.
- Rules: a workspace added here follows the rules of
  `apps/website/frontend/src/v2/apps/`: it declares its module in that folder's `mod.rs`, imports
  from `crate::v2::core` and `website_map_engine`, and never from a page or a sibling workspace;
  its route needs a row in both `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`.

## Related documentation

- [Mission planner](/documentation_v2/website/frontend/apps/planner/mission_planner.md) — the planned workspace's design notes and open work.
