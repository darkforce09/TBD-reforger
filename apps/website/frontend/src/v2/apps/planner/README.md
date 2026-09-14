# 2. Mission Planner (`src/v2/planner`)

The pre-match tactical whiteboard and operational briefing suite.

---

## Purpose
Allows commanders, squad leaders, and platoon leadership to draw operational battle plans on top of a published scenario before boots hit the ground.

## Architecture
- **Consumes:** Mounts `<MapCanvas />` from `src/v2/map_engine`.
- **`ui/`**: Tactical whiteboard chrome (Floating drawing palette, operational phase selector, slide-out commander intent drawer).
- **`features/`**: Tactical phase lines, assault arrows, recon markers, squad sector tasking, radio plan assignments.
- **`state/`**: Operational plan overlay store (lightweight vector drawing format saved alongside event briefings).
