# After-action review workspace

The folder reserved for the after-action review workspace, a replay of a finished match from
server telemetry over a read-only map. It holds no code: only this README.

## Contents

```text
apps/frontend/src/workspaces/aar/
```

## Routes

None: the folder serves no route, and `apps/frontend/src/app_routes.rs` has no replay
route.

## Public surface

None: `apps/frontend/src/workspaces/mod.rs` declares no `aar` module, so nothing here
compiles.

## Boundaries

- Depends on: nothing.
- Used by: nothing.
- Rules: a workspace added here follows the rules of
  `apps/frontend/src/workspaces/`: it declares its module in that folder's `mod.rs`, imports
  from `crate::foundation` and the map crates, and never from a page or a sibling workspace;
  its route needs a row in both `apps/frontend/src/app_routes.rs` and
  `apps/frontend/src/foundation/route_table/mod.rs`.

## Related documentation

- [After-action review](/documentation/apps/frontend/workspaces/aar/after_action_review.md) — the planned replay's design notes, the telemetry it needs and its open work.
