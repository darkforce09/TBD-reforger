# Viewshed job scheduler

One live visibility job per tool, advanced in budgeted batches and cancelled by the next placement:
the terrain disc of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
line-of-sight tool and the per-floor visibility wash of a building.

## Contents

```text
apps/website/map-engine/src/editing/tools/viewshed_scheduler/
├── host.rs          `SchedulerHost`: the clock, refusal sink, frame pump and completion signal
├── lanes.rs         per-tool slots, generation counter, `cancel`, progress, last refusal
├── mod.rs           the module tree; re-exports the scheduler's surface
├── terrain_lane.rs  `submit_terrain` and `pump_terrain_once`: the terrain disc, published when done
├── tests/           unit tests for lane isolation, cap refusal and sync parity
└── wash_lane.rs     `submit_wash`, `step_wash` and `take_wash`: the building wash its owner steps
```

## How it works

```text
submit_terrain(x, y)                         submit_wash(level, eye_y, obs, params)
  next generation, cancel Terrain lane         next generation, cancel BuildingWash lane
  ViewshedJob::new (cap check)                 WashJob::new (cap check)
  one 4 ms batch, publish the raster so far    park the job
  park the job, request_pump()               owner: step_wash(blocked) per frame, then take_wash
host frame: pump_terrain_once()
  observer no longer placed → drop the job
  one batch; done → publish, on_terrain_finished()
```

Each lane holds at most one job (`ViewshedTool::Terrain`, `ViewshedTool::BuildingWash`), and a
submit cancels only its own lane's predecessor, before any work. Every submit bumps one monotonic
generation, which stamps the job it creates. A batch runs for at most `VIEWSHED_BUDGET_MS` (4 ms)
of the host clock.

The terrain lane parks everything a pump needs to finish without its submitter: the job, the
observer and the DEM sampler registered with the line-of-sight tool. It builds the job with the
line-of-sight tool's eye height, radius, cell size and Everon manifest, publishes each raster into
the registered viewshed state, and drops a job whose observer is no longer the placed one, so a
dismissed disc never comes back. The wash lane cannot park its `blocked` closure, which borrows a
building's geometry for the call, so the owner passes it to every `step_wash`.

`install_host` replaces the host services wholesale. The default table reports nothing, pumps
nothing and keeps a fallback clock that always advances (wall time natively, a tick counter on
`wasm32`), so a budget loop ends even with no host. Natively, `submit_terrain` runs the job to
completion before it returns; on `wasm32` it runs one batch and the rest waits for the host's pump.
A refused request records the cap's message in `last_refusal`, which a later success never clears.

## Boundaries

- Depends on: `crate::spatial::los::terrain::scheduler::ViewshedJob` and
  `crate::spatial::los::terrain::viewshed` (`ViewshedParams`, `ViewshedCapRefused`);
  `crate::spatial::los::interior::wash` (`WashJob`, `WashParams`, `LevelWash`); and
  `crate::editing::tools::line_of_sight` (`host_registry`, `terrain_survey`, `terrain_verdict`).
- Used by: `crate::editing::tools::line_of_sight::viewshed_texture::place_viewshed`, which submits
  the terrain disc; the Mission Creator's scheduler host
  (`apps/website/frontend/src/v2/apps/editor/input/tools/viewshed_scheduler.rs`), which installs
  the services and pumps the terrain lane, installed by the canvas mount in
  `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount.rs`. The wash lane and the
  readouts (`last_refusal`, `active_generation`, `progress`) have no caller outside `tests/`.
- Rules: one job per tool, and a submit cancels only its own tool's job
  (`one_active_job_per_tool_and_a_submit_cancels_its_own` in `tests/lane_isolation.rs`); a
  scheduled wash equals the synchronous one (`a_scheduled_wash_finishes_equal_to_the_sync_path`);
  an over-cap request is refused with a message (`an_over_cap_submit_is_refused_with_a_message`).
