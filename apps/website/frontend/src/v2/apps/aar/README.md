# After-action review workspace

The folder reserved for the after-action review workspace, a replay of a finished match from
server telemetry over a read-only map. It holds no code: only this README.

## Contents

```text
apps/website/frontend/src/v2/apps/aar/
```

## Routes

None: the folder serves no route, and `apps/website/frontend/src/app_routes.rs` has no replay
route.

## Public surface

None: `apps/website/frontend/src/v2/apps/mod.rs` declares no `aar` module, so nothing here
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

- [After-action review](/documentation_v2/website/frontend/apps/aar/after_action_review.md) — the planned replay's design notes, the telemetry it needs and its open work.
