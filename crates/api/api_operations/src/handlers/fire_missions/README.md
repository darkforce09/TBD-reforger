# Fire-mission handlers

The HTTP handlers of saved fire missions: the save the mortar calculator sends, which the server
re-solves against the pinned ballistics catalog before storing it, and the list of an
[event](/documentation/glossary/a_to_f.md#event)'s saved fire missions.

## Contents

```text
crates/api/api_operations/src/handlers/fire_missions/
├── mod.rs   the module tree and the event access check both routes share
├── list.rs  an event's fire missions, legacy and catalog-model, oldest first
└── save.rs  the strict save body, its range checks, the re-solve and the transactional store
```

## How it works

| Route | Tier | Answer |
|---|---|---|
| `POST /api/v1/fire-missions` | member | 201 `SavedFireMission`; 400; 403; 404 unknown catalog version or event; 422 with `details.code` |
| `GET /api/v1/events/{id}/fire-missions` | member | 200 `FireMissionList`; 400 malformed id; 403; 404 |

- `save.rs` decodes the `FireMissionSave` body strictly (an unknown field, such as the retired
  single-tube `weapon_system`, is a 400), refuses what the columns constrain and the solver
  accepts (`catalog_version` 0, a battery of more than 12 guns, a blank `target_grid`, a wind
  outside `[0, 360)` degrees or with a negative speed), then calls `services::fire_mission_resolve::resolve_fire_mission` and
  `services::fire_mission_store::insert_fire_mission`. The answer carries the server's solution,
  never the client's.
- The 422 codes: `solution_mismatch` (the client solution differs from the re-solve by more
  than 1 mil in the weapon's convention or 0.1 s, or in any discrete field; `details.mismatches`
  lists each with both values, `details.server_solution` is the re-solve),
  `fire_mission_refused` (the assembler refuses the inputs, for example a burst height on a
  shell without a time fuze) and `no_firing_solution` (the lead gun's fired charge does not
  solve).
- Both routes admit a caller to an event through
  `services::event_access::visibility::viewer_event_access`: an event that is missing, deleted
  or hidden from the caller answers the same `404 event not found`, and a partial viewer (one
  admitted only by squad or slot policies) 403. A save with no `event_id` names no event.
- `list.rs` answers legacy rows, including weapons no catalog solves such as `M120 120mm`, with
  every catalog-model field `null` and `guns` empty: they cannot be re-solved.

## Boundaries

- Depends on: `api_operations::services::fire_mission_resolve`, `api_operations::services::fire_mission_store`
  and `api_operations::services::event_access`; `api_operations::models::fire_mission`;
  `fire_mission_planning::fire_mission` for the body's input and
  solution types; `api_http_layer` for `AuthUser`; `api_foundation` for `PathParams` and
  `ApiError`.
- Used by: `crates/api/api_operations/src/routes.rs`; over HTTP, the mortar calculator in
  `apps/frontend/src/pages/field_tools/mortar/`; the tests
  `apps/api/tests/game_ballistics_fire_missions.rs` and
  `apps/api/tests/fire_mission_solution.rs`.
- Rules: both routes take `AuthUser`; a refused save stores nothing.

## Related documentation

- [Fire-mission contract](/contracts/definitions/fire-mission.schema.json) — the save body,
  the solution, the stored mission and the list.
- [Ballistics catalog handlers](../ballistics_catalogs/README.md) — the catalogs a save pins.
