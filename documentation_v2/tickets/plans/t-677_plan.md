# T-677 — Plan

## Context

`TBD_SpawnManager.c` (:963, :1166) spawns every body with AI disabled, so no waypoint can have a subject. Operator decision 2026-08-02: AI units are coming. Nine ATTR-FIELD-WP ids plus six interaction keys (CONN-WP, KEY-WP, ACTION-WP) are on the wire since T-706. T-678 (group AI state) shares this gate and packs after.

## Approach

1. Verify on main: the two spawn sites pass AI disabled unconditionally.
2. `TBD_SpawnManager.c`: enable AI for groups whose payload carries waypoints (or the T-678 attrs); players unchanged.
3. New `AI/TBD_WaypointRuntime.c`: per-group ordered waypoint queue applying type, completion radius, timeout and the interaction connections after spawn.

## Risks

- Dedicated-server AI pathing is unobservable headlessly — human checklist.
- Enabling AI for unwaypointed groups would change existing missions; gate strictly on payload presence.

## Verification

- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-677`
