# Systems/Audio

In-world positional sound emitters and event-driven musical cues.

### Roles & Responsibilities
- **`TBD_AudioEmitter.c`:**
  - Reads `audio` specifications from the compiled mission document.
  - Spawns client-side `TBD_AudioSourceEntity` instances at authored 3D coordinates.
  - Performs distance checks against local player positions to dynamically raise or lower sound volumes.
  - Triggers one-shot or looping music cues on stage events (`mission_start`, `task_succeeded`, `task_failed`, `mission_end`).

### Call Flow & Contracts
Evaluated server-side; playback instructions are delivered to each client's `SCR_PlayerController` via targeted RPCs for local spatial playback.
