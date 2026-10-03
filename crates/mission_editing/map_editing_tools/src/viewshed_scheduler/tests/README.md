# Viewshed scheduler tests

Unit tests of the viewshed job scheduler: one live job per tool with a submit cancelling only its
own tool's job, a scheduled wash equal to the synchronous one, an over-cap refusal with its
message, and the terrain lane declining without a registered sampler.

## Contents

```text
crates/mission_editing/map_editing_tools/src/viewshed_scheduler/tests/
└── lane_isolation.rs  lane isolation, budget slicing, cap refusal and decline without a sampler
```

## Boundaries

- Depends on: the scheduler (mounted from `viewshed_scheduler/mod.rs` with a `#[path]` attribute)
  and `interior_line_of_sight::floor_wash`, `terrain_line_of_sight::viewshed`.
- Used by: `cargo test -p map_editing_tools`.
- Rules: the tests run on the default host table, so budgets are measured with
  `time_source::monotonic_ms`, and on a native target a terrain disc completes inside its submit.
