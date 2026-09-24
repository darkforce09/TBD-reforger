# T-936.5 — Plan

## Context
No audio vocabulary in mission.schema.json, no audio panel, no audio script under Scripts/Game/TBD
(audit S1). Missions carry no authored sound.

## Approach
1. Schema `audio {emitters[], musicCues[]}`; golden updated.
2. `mission/audio.rs` (new, in mission/mod.rs): model + validator (radius, events, ids); register in extensions.rs.
3. `panels/audio_emitters.rs` (new, in panels/mod.rs): place-on-map via the marker gesture, fields, cue table.
4. `Gamemode/TBD_AudioEmitter.c` (new): sound source per emitter, cue playback on events.
5. Perturbation: accept radius 0 → validator test red; restore, touch, green.

## Risks
- Sound asset ids must resolve in the mod — validate against a known list, warn on unknown.
- Placement gesture reuse must not regress marker placement (leptos-gates).

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask ci schema-validate`; `cargo xtask mk ci-local-leptos`
- `cargo xtask mk leptos-gates`; `cargo xtask mod compile`; `cargo xtask platform wave gate --slice T-936.5`
