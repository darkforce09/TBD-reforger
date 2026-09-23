# `wave_plan/`

## Responsibility

Reads the recorded wave plan, projects its lanes exactly as stored, and explains ownership-path collisions.

## Public surface

`services::lock_file` parses the lock and evaluates ownership prefix collisions. `models::wave_projection::WavesModel` builds lanes and unplanned-ticket projections. `WavePlanView` supplies display state; `WavePlanEvent` reports selection, copying, and expansion.

## Dependency rules

Consumes registry models and services, not browser UI. Missing or malformed locks are local refusal states and do not invalidate the registry board. Filtering dims unmatched members without removing or reordering recorded lanes. The feature never recalculates packing.

## Files

- [events.rs](events.rs) — Events.
- [mod.rs](mod.rs) — Module interface and composition.
- [models/mod.rs](models/mod.rs) — Module interface and composition.
- [models/tests/wave_projection.rs](models/tests/wave_projection.rs) — Tests for wave projection.
- [models/view.rs](models/view.rs) — View.
- [models/wave_projection.rs](models/wave_projection.rs) — Wave projection.
- [services/lock_file.rs](services/lock_file.rs) — Lock file.
- [services/mod.rs](services/mod.rs) — Module interface and composition.
- [services/tests/lock_file.rs](services/tests/lock_file.rs) — Tests for lock file.
- [ui/lane_view.rs](ui/lane_view.rs) — Lane view.
- [ui/mod.rs](ui/mod.rs) — Module interface and composition.

Unit tests live in sibling `tests/` files declared with `#[cfg(test)]` and an explicit `#[path = "tests/…"]`. Production files contain fewer than 500 raw lines; test files contain at most 1,000.
