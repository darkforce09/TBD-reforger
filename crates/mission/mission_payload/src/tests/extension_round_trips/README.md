# Extension round trips

Payload tests that compile a mission carrying one authored extension block and check where the
block lands: promoted out of the environment bag onto the payload root, absent when nothing is
authored, and read back by the extension carrier.

## Contents

```text
extension_round_trips/
├── mod.rs                the shared imports: the payload compiler, the block registry, `serde_json`
├── audio.rs              the `audio` block
├── radio_plan.rs         the `radioPlan` block
├── spawn_modules.rs      the `spawnModules` block
├── tactical_graphics.rs  the `tacticalGraphics` block
├── tasks.rs              the `tasks` block
└── weather_timeline.rs   the `weatherTimeline` block
```

## How it works

Each file repeats the fixtures of its block's own tests in the mission model, so a fixture change
there that is not made here shows as a failing round trip rather than a silent drift.
