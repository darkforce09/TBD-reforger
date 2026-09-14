# Standalone Map Applications (`src/v2/apps`)

This directory houses the 3 independent full-screen applications that consume the shared map engine (`src/v2/map_engine`).

---

## The 3 Applications

```text
apps/
├── editor/    <-- 1. SCENARIO CREATOR (Eden CAD Workspace)
├── planner/   <-- 2. MISSION PLANNER (Tactical Whiteboard)
└── aar/       <-- 3. AFTER-ACTION REPORT (Telemetry Replay Player)
```

Each application owns 100% of its own UI, layouts, feature logic, and state pipeline:
1. **`editor/`:** Scenario authoring with its own 2-row top strip, ORBAT tree dock, asset palette dock, and Yrs CRDT document store.
2. **`planner/`:** Pre-mission tactical whiteboard with operational phase selector, assault arrow tools, and squad tasking cards.
3. **`aar/`:** Post-mission forensics with its own media player playback deck, timeline scrubber bar, killfeed ticker, and casualty inspector.
