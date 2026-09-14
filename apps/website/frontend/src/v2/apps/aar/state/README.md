# AAR State (`aar/state`)

## Responsibilities
- Binary/JSON telemetry log chunk ingestion.
- Playback clock state machine (Current playback timestamp, paused state, playback speed multiplier).
- Entity interpolation cache for smooth 60fps movement between 1Hz server telemetry ticks.
