# T-936.4 — Plan

## Context
environment.weatherPreset (mission.schema.json:190) is one static preset edited by panels/env.rs. Nothing
changes weather during a mission; no weather script exists under Gamemode/.

## Approach
1. Schema `weatherTimeline.keyframes[]` {atMinutes, weatherPreset, windDirDeg?, fog?}; golden updated.
2. `mission/weather.rs` (new, in mission/mod.rs): model + strict ordering validator; register in extensions.rs.
3. `panels/weather_timeline.rs` (new, in panels/mod.rs): keyframe editor, undoable.
4. `Gamemode/TBD_WeatherRuntime.c` (new): apply keyframes at offsets via the world weather manager.
5. Perturbation: accept equal atMinutes → ordering test red; restore, touch, green.

## Risks
- Preset vocabulary must match environment.weatherPreset — share one list in weather.rs.
- Weather manager API differences per world — log every transition for the checklist.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask ci schema-validate`; `cargo xtask mk ci-local-leptos`
- `cargo xtask mk leptos-gates`; `cargo xtask mod compile`; `cargo xtask platform wave gate --slice T-936.4`
