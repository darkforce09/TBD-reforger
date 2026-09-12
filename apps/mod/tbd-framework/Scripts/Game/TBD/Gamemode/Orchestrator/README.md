# Gamemode/Orchestrator

The central conductor of the TBD match lifecycle.

---

## Contents

- **`TBD_FrameworkManager.c`:** The primary `SCR_BaseGameModeComponent` attached to `TBD_GameMode.et`.

---

## Responsibilities

1. **Stage State Machine:**
   Coordinates transitions across all stages: `LOADING` &rarr; `LOBBY` &rarr; `BRIEFING` &rarr; `LIVE` &rarr; `END` &rarr; `DEBRIEF`. Replicates stage changes to clients and triggers local UI screens.
2. **Round Clock & Ticking:**
   Manages the authoritative match countdown timer (`timeLimitSeconds`), periodic time broadcasts ("10 minutes remaining"), and triggers the round-over evaluation when time expires.
3. **Flow Parameters (`doc.flow`):**
   Parses and applies author-specified match parameters from the compiled mission document via `TBD_MissionFlow` (safestart seconds, round time limits, and join-in-progress policies).
4. **End & Debrief Presentation:**
   Takes authoritative snapshots upon round termination (`ResolveEndWinner`, `SnapshotEndBanner`, `SnapshotDebriefBoard`) to drive post-game victory screens and after-action review scoreboards.

---

## Roadmap: Modular Decomposition

`TBD_FrameworkManager.c` is currently 1,746 lines. In a future pass, its internal responsibilities can be decomposed into focused companion classes:
- `TBD_RoundClock.c` (clock ticking & broadcast logic)
- `TBD_StageCoordinator.c` (stage transitions & client UI dispatch)
- `TBD_EndBannerService.c` (post-game snapshot and winner resolution)
